use std::fmt;
use crate::storage::Album;

#[derive(Debug)]
pub enum QueueError {
    NoDeezerTab,
    NoBrowserDebugPort,
    ScriptError(String),
    CdpError(String),
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueError::NoDeezerTab => write!(f, "no Deezer tab found in any supported browser"),
            QueueError::NoBrowserDebugPort => write!(
                f,
                "no browser with --remote-debugging-port found; \
                 launch Chrome/Chromium with --remote-debugging-port=9222"
            ),
            QueueError::ScriptError(msg) => write!(f, "{msg}"),
            QueueError::CdpError(msg) => write!(f, "CDP error: {msg}"),
        }
    }
}

impl std::error::Error for QueueError {}

type WsStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

/// Scan /proc/*/cmdline for `--remote-debugging-port=PORT`.
fn find_debug_port() -> Option<u16> {
    std::fs::read_dir("/proc")
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let mut p = e.path();
            p.push("cmdline");
            std::fs::read(p).ok()
        })
        .find_map(|cmdline| {
            cmdline.split(|&b| b == 0 || b == b' ').find_map(|arg| {
                let s = std::str::from_utf8(arg).ok()?;
                s.strip_prefix("--remote-debugging-port=")
                    .and_then(|p| p.parse::<u16>().ok())
                    .filter(|&port| port != 0)
            })
        })
}

/// Send a `Runtime.evaluate` CDP command and wait for the matching response.
async fn cdp_eval(ws: &mut WsStream, id: u32, expr: &str) -> Result<serde_json::Value, QueueError> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let msg = serde_json::json!({
        "id": id,
        "method": "Runtime.evaluate",
        "params": {"expression": expr, "returnByValue": true}
    })
    .to_string();

    ws.send(Message::Text(msg))
        .await
        .map_err(|e| QueueError::CdpError(e.to_string()))?;

    loop {
        match ws.next().await {
            Some(Ok(Message::Text(text))) => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                    if v.get("id").and_then(|i| i.as_u64()) == Some(u64::from(id)) {
                        return Ok(v);
                    }
                }
            }
            Some(Ok(_)) => continue,
            Some(Err(e)) => return Err(QueueError::CdpError(e.to_string())),
            None => {
                return Err(QueueError::CdpError(
                    "WebSocket closed unexpectedly".to_string(),
                ))
            }
        }
    }
}

pub async fn add_to_queue(album: &Album, debug: bool) -> Result<(), QueueError> {
    use std::time::Duration;
    use tokio::time::sleep;
    use tokio_tungstenite::connect_async;

    // 1. Find browser debug port
    let port = find_debug_port().ok_or(QueueError::NoBrowserDebugPort)?;

    // 2. List open tabs
    let tabs: Vec<serde_json::Value> = reqwest::get(format!("http://localhost:{port}/json"))
        .await
        .map_err(|e| QueueError::CdpError(e.to_string()))?
        .json()
        .await
        .map_err(|e| QueueError::CdpError(e.to_string()))?;

    // 3. Find Deezer tab
    let tab = tabs
        .iter()
        .find(|t| {
            t["url"]
                .as_str()
                .map(|u| u.contains("deezer.com"))
                .unwrap_or(false)
        })
        .ok_or(QueueError::NoDeezerTab)?;

    let ws_url = tab["webSocketDebuggerUrl"]
        .as_str()
        .ok_or(QueueError::NoDeezerTab)?
        .to_string();

    // 4. Connect via WebSocket
    let (mut ws, _) = connect_async(&ws_url)
        .await
        .map_err(|e| QueueError::CdpError(e.to_string()))?;

    // CDP Runtime.evaluate runs in the main world — no blob injection needed.
    let main_world_js = include_str!("js/queue_main_world.js")
        .replace("__ALBUM_ID__", &album.id.to_string());

    const CLEAR_JS: &str =
        "['__dz_running','__deezranbum','__deezranbum_logs'].forEach(k=>localStorage.removeItem(k))";

    // 5. Clear previous state, then run the queue logic
    cdp_eval(&mut ws, 1, CLEAR_JS).await?;
    cdp_eval(&mut ws, 2, &main_world_js).await?;

    // 6. Poll localStorage for the result (up to ~18 s)
    sleep(Duration::from_millis(500)).await;
    let mut payload: Option<String> = None;
    for i in 0..60_u32 {
        sleep(Duration::from_millis(300)).await;
        let r = cdp_eval(&mut ws, 3 + i, "localStorage.getItem('__deezranbum')").await?;
        // CDP returns {type:"string", value:"..."} for a string, {type:"object", subtype:"null"} for null
        let result_obj = &r["result"]["result"];
        if result_obj["type"].as_str() == Some("string") {
            if let Some(v) = result_obj["value"].as_str() {
                payload = Some(v.to_string());
                break;
            }
        }
    }

    cdp_eval(&mut ws, 64, CLEAR_JS).await.ok();

    let payload = match payload {
        Some(p) => p,
        None => {
            let logs = cdp_eval(&mut ws, 65, "localStorage.getItem('__deezranbum_logs')")
                .await
                .ok()
                .and_then(|r| r["result"]["result"]["value"].as_str().map(String::from))
                .unwrap_or_default();
            return Err(QueueError::ScriptError(format!(
                "timeout waiting for JS result. logs:\n{logs}"
            )));
        }
    };

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload) {
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
        let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("ok");
        if status != "ok" {
            return Err(QueueError::ScriptError(format!("js reported status={status}")));
        }
    }

    Ok(())
}
