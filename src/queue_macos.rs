use std::fmt;
use crate::storage::Album;

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

fn build_js(album_id: u64) -> String {
    // tab.execute() runs in an isolated world; inject a blob <script> to reach the
    // main world where window.dzPlayer lives.
    // Layer order: queue_outer.js (JXA) → execute(queue_inject.js) → blob <script>(queue_main_world.js)
    let main_world_js = include_str!("js/queue_main_world.js")
        .replace("__ALBUM_ID__", &album_id.to_string());
    let main_world_js_json =
        serde_json::to_string(&main_world_js).expect("failed to JSON-encode main world JS");
    let inject_js = include_str!("js/queue_inject.js")
        .replace("__MAIN_WORLD_JS_JSON__", &main_world_js_json);
    let inject_js_json =
        serde_json::to_string(&inject_js).expect("failed to JSON-encode inject JS");
    include_str!("js/queue_outer.js").replace("__MAIN_WORLD_JS_JSON__", &inject_js_json)
}

pub async fn add_to_queue(album: &Album, debug: bool) -> Result<(), QueueError> {
    use std::fs;
    use tempfile::NamedTempFile;
    use tokio::process::Command;

    let js = build_js(album.id);
    let file = NamedTempFile::new().map_err(QueueError::SpawnFailed)?;
    fs::write(file.path(), &js).map_err(QueueError::SpawnFailed)?;

    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg(file.path())
        .output()
        .await
        .map_err(QueueError::SpawnFailed)?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        eprintln!("[osascript stderr]: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.trim().splitn(2, '\n');
    let first = lines.next();
    let payload = lines.next().unwrap_or("").trim();

    if !payload.is_empty() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
            if debug {
                let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("?");
                eprintln!("[queue] js status: {status}");
                if let Some(logs) = v.get("logs").and_then(|l| l.as_str()) {
                    if !logs.is_empty() {
                        eprintln!("[queue] js logs:\n{logs}");
                    }
                }
                if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
                    eprintln!("[queue] js error: {err}");
                }
            }
        }
    }

    match first {
        Some("ERROR:NO_DEEZER_TAB") => Err(QueueError::NoDeezerTab),
        Some(line) if line.starts_with("ERROR:") => {
            Err(QueueError::ScriptError(line.to_string()))
        }
        Some("OK") => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("ok");
                if status != "ok" {
                    return Err(QueueError::ScriptError(format!("js reported status={status}")));
                }
            }
            Ok(())
        }
        other => {
            eprintln!("[queue debug] unexpected output: {other:?}");
            Ok(())
        }
    }
}
