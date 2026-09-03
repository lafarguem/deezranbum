use std::fmt;
use std::fs;
use std::process::Command;
use tempfile::NamedTempFile;

use crate::storage::{Album, ItemKind};

#[derive(Debug)]
pub enum QueueError {
    NoDeezerTab,
    SpawnFailed(std::io::Error),
    ScriptError(String),
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueError::NoDeezerTab => write!(f, "no Deezer tab found in any supported browser"),
            QueueError::SpawnFailed(e) => write!(f, "failed to spawn osascript: {e}"),
            QueueError::ScriptError(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for QueueError {}

fn build_js(album: &Album) -> String {
    // tab.execute() in JXA runs in an isolated world — page JS globals like
    // window.dzPlayer are invisible there. To reach the main world we inject a
    // <script src=blob:...> via queue_inject.js; the blob URL bypasses CSP and
    // runs queue_main_world.js in the real page context.
    //
    // Layer order: queue_outer.js (JXA) → execute(queue_inject.js) → blob <script>(queue_main_world.js)

    let id = album.queue_id();
    let (method, body, object_type) = match album.kind {
        ItemKind::Album => (
            "deezer.pageAlbum",
            format!("{{alb_id: {id}, lang: 'us', tab: 0, header: true}}"),
            "album",
        ),
        ItemKind::Playlist => {
            let nb = album.nb_tracks.unwrap_or(2000).clamp(100, 10000);
            (
                "deezer.pagePlaylist",
                format!(
                    "{{playlist_id: '{id}', lang: 'us', nb: {nb}, start: 0, tab: 0, tags: true, header: true}}"
                ),
                "playlist",
            )
        }
    };

    let main_world_js = include_str!("js/queue_main_world.js")
        .replace("__GW_METHOD__", method)
        .replace("__GW_BODY__", &body)
        .replace("__OBJECT_TYPE__", object_type)
        .replace("__OBJECT_ID__", &id.to_string());

    let main_world_js_json =
        serde_json::to_string(&main_world_js).expect("failed to JSON-encode main world JS");

    let inject_js =
        include_str!("js/queue_inject.js").replace("__MAIN_WORLD_JS_JSON__", &main_world_js_json);

    let inject_js_json =
        serde_json::to_string(&inject_js).expect("failed to JSON-encode inject JS");

    include_str!("js/queue_outer.js").replace("__MAIN_WORLD_JS_JSON__", &inject_js_json)
}

pub fn add_to_queue(album: &Album, debug: bool) -> Result<(), QueueError> {
    let js = build_js(album);

    let file = NamedTempFile::new().map_err(QueueError::SpawnFailed)?;

    fs::write(file.path(), js).map_err(QueueError::SpawnFailed)?;

    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg(file.path())
        .output()
        .map_err(QueueError::SpawnFailed)?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        eprintln!("[osascript stderr]: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.trim().splitn(2, '\n');
    let first = lines.next();
    let payload = lines.next().unwrap_or("").trim();

    // Parse JS payload; print debug info only when --debug is set.
    if !payload.is_empty()
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(payload)
        && debug
    {
        let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("?");
        eprintln!("[queue] js status: {status}");
        if let Some(logs) = v.get("logs").and_then(|l| l.as_str())
            && !logs.is_empty()
        {
            eprintln!("[queue] js logs:\n{logs}");
        }

        if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
            eprintln!("[queue] js error: {err}");
        }
    }

    match first {
        Some("ERROR:NO_DEEZER_TAB") => return Err(QueueError::NoDeezerTab),
        Some(line) if line.starts_with("ERROR:") => {
            return Err(QueueError::ScriptError(line.to_string()));
        }
        Some("OK") => {
            // Even on OK, the JS status might be non-ok (e.g. timeout/main-rejection).
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("ok");
                if status != "ok" {
                    return Err(QueueError::ScriptError(format!(
                        "js reported status={status}"
                    )));
                }
            }
        }
        other => eprintln!("[queue debug] unexpected output: {other:?}"),
    }
    Ok(())
}
