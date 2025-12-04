mod session;

use axum::extract::{Query, ws::{Message, WebSocket, WebSocketUpgrade}};
use axum::{routing::get, Router, response::{Html, IntoResponse}, http::StatusCode};
use axum::extract::State;
use futures::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use qrcode::QrCode;
use qrcode::render::svg;
use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixDatagram;
use std::os::unix::prelude::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use session::SessionManager;

#[derive(Deserialize)]
struct AuthQuery {
    token: Option<String>,
}

#[derive(Clone)]
struct AppState {
    auth_token: Option<String>,
    session_manager: SessionManager,
    host: String,
    port: u16,
    public_url: Option<String>,
}

#[tokio::main]
async fn main() {
    // Load configuration from environment
    let auth_token = std::env::var("BRIDGE_AUTH_TOKEN").ok();
    let timeout = std::env::var("BRIDGE_SESSION_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800); // 30 minutes default
    let host = std::env::var("BRIDGE_HOST").unwrap_or("0.0.0.0".to_string());
    let port: u16 = std::env::var("BRIDGE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7777);
    let public_url = std::env::var("BRIDGE_PUBLIC_URL").ok();

    let state = AppState {
        auth_token: auth_token.clone(),
        session_manager: SessionManager::new(timeout),
        host: host.clone(),
        port,
        public_url: public_url.clone(),
    };

    // Spawn background task to cleanup expired sessions
    let session_cleanup = state.session_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // Every 5 min
        loop {
            interval.tick().await;
            let removed = session_cleanup.cleanup_expired().await;
            if removed > 0 {
                eprintln!("Cleaned up {} expired sessions", removed);
            }
        }
    });

    let app = Router::new()
        .route("/bridge", get(ws_handler))
        .route("/qr", get(qr_handler))
        .route("/sessions", get(sessions_handler))
        .nest_service("/", ServeDir::new("static"))
        .with_state(state.clone());

    let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse().unwrap();
    let default_url = format!("http://{}:{}", host, port);
    let display_url = public_url.as_ref().map(|u| u.as_str()).unwrap_or(&default_url);
    
    if auth_token.is_some() {
        println!("🔒 Bridge listening at {}  (Authentication ENABLED)", display_url);
        println!("   📱 QR Code available at {}/qr", display_url);
    } else {
        println!("⚠️  Bridge listening at {}  (Authentication DISABLED - Set BRIDGE_AUTH_TOKEN to enable)", display_url);
    }
    println!("   Bind address: {}", addr);
    println!("   Static files: ./static");
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn qr_handler(State(state): State<AppState>) -> impl IntoResponse {
    // Generate QR code with connection URL + token
    let token = match &state.auth_token {
        Some(t) => t.clone(),
        None => return (StatusCode::BAD_REQUEST, Html("<html><body><h1>Authentication not enabled</h1><p>Set BRIDGE_AUTH_TOKEN environment variable to enable QR code generation.</p></body></html>")).into_response(),
    };

    // Determine the connection URL - use public URL if set, otherwise construct from host:port
    // Point to root (/) instead of /bridge so browser loads the HTML page first
    let base_url = state.public_url.as_ref()
        .map(|u| u.trim_end_matches('/').to_string())
        .unwrap_or_else(|| format!("http://{}:{}", state.host, state.port));
    
    let connection_url = format!("{}/?token={}", base_url, token);
    
    // Generate QR code
    let qr = match QrCode::new(connection_url.as_bytes()) {
        Ok(qr) => qr,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("<html><body><h1>Error</h1><p>Failed to generate QR code: {}</p></body></html>", e))).into_response(),
    };

    let svg = qr.render::<svg::Color>()
        .min_dimensions(400, 400)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();

    let html = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Bridge - QR Code</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            background: #0f1115;
            color: #e6e8eb;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            padding: 20px;
        }}
        .container {{
            text-align: center;
            max-width: 600px;
        }}
        h1 {{
            margin-bottom: 10px;
        }}
        p {{
            color: #9aa4b2;
            margin-bottom: 30px;
        }}
        .qr-wrapper {{
            background: white;
            padding: 30px;
            border-radius: 16px;
            display: inline-block;
            box-shadow: 0 4px 6px rgba(0,0,0,0.3);
        }}
        .connection-url {{
            margin-top: 20px;
            padding: 15px;
            background: #1b1f2a;
            border-radius: 8px;
            word-break: break-all;
            font-family: monospace;
            font-size: 12px;
        }}
        .instructions {{
            margin-top: 30px;
            text-align: left;
            background: #1b1f2a;
            padding: 20px;
            border-radius: 8px;
        }}
        .instructions h2 {{
            margin-top: 0;
            font-size: 18px;
        }}
        .instructions ol {{
            padding-left: 20px;
        }}
        .instructions li {{
            margin-bottom: 10px;
            line-height: 1.6;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🔗 Bridge Connection QR Code</h1>
        <p>Scan this QR code with your mobile device to connect instantly</p>
        <div class="qr-wrapper">
            {}
        </div>
        <div class="connection-url">
            <strong>Connection URL:</strong><br/>
            {}
        </div>
        <div class="instructions">
            <h2>📱 How to Connect</h2>
            <ol>
                <li>Open your mobile browser (Safari on iOS, Chrome on Android)</li>
                <li>Scan this QR code using your camera app</li>
                <li>Tap the notification to open the link</li>
                <li>You'll be automatically connected to your terminal!</li>
            </ol>
            <p style="color: #9aa4b2; font-size: 14px;"><em>Note: Make sure both devices are on the same Tailscale network or VPN.</em></p>
        </div>
    </div>
</body>
</html>
    "#, svg, connection_url);

    Html(html).into_response()
}

async fn sessions_handler(State(state): State<AppState>) -> impl IntoResponse {
    let sessions = state.session_manager.list_sessions().await;
    let response = serde_json::json!({
        "active_sessions": sessions.len(),
        "sessions": sessions,
    });
    axum::Json(response).into_response()
}

async fn ws_handler(
    State(state): State<AppState>,
    Query(auth): Query<AuthQuery>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    // Validate token if authentication is enabled
    if let Some(expected_token) = &state.auth_token {
        match &auth.token {
            Some(provided_token) if provided_token == expected_token => {
                // Token valid, proceed
            }
            _ => {
                return (StatusCode::UNAUTHORIZED, "Invalid or missing token").into_response();
            }
        }
    }

    // Create session
    let session_manager = state.session_manager.clone();
    let session = session_manager.create_session(None, None).await;
    let session_id = session.id.clone();
    
    eprintln!("[{}] New WebSocket connection", session_id);

    ws.on_upgrade(move |socket| handle_socket(socket, session_manager, session_id))
}

async fn handle_socket(socket: WebSocket, session_manager: SessionManager, session_id: String) {
    // --- PTY setup -----------------------------------------------------------
    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows: 40,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(p) => p,
        Err(e) => { eprintln!("openpty: {e}"); return; }
    };

    // Per-connection Unix datagram socket for shell hooks
    let sock_path = make_sock_path();
    let uds = match UnixDatagram::bind(&sock_path) {
        Ok(s) => s,
        Err(e) => { eprintln!("bind uds: {e}"); return; }
    };
    let _ = fs::set_permissions(&sock_path, fs::Permissions::from_mode(0o600));

    let mut cmd = CommandBuilder::new(std::env::var("BRIDGE_SHELL").unwrap_or("/bin/zsh".into()));
    cmd.env("BRIDGE_SOCK", sock_path.to_string_lossy().to_string()); // zsh hooks use this

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => { eprintln!("spawn shell: {e}"); let _=fs::remove_file(&sock_path); return; }
    };
    drop(pair.slave);

    // Keep master for read/write + resize
    let master = pair.master;
    let mut reader = match master.try_clone_reader() {
        Ok(r) => r, Err(e) => { eprintln!("clone reader: {e}"); let _=fs::remove_file(&sock_path); return; }
    };
    let writer = match master.take_writer() {
        Ok(w) => w, Err(e) => { eprintln!("take writer: {e}"); let _=fs::remove_file(&sock_path); return; }
    };
    let writer = Arc::new(Mutex::new(writer));
    let master_arc = Arc::new(Mutex::new(master));

    let (ws_tx, mut ws_rx) = socket.split();
    let ws_tx_arc = Arc::new(Mutex::new(ws_tx));

    // PTY -> WS bytes via channel
    let (send_bytes, recv_bytes) = crossbeam_channel::unbounded::<Vec<u8>>();
    let _reader_thread = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => { let _ = send_bytes.send(Vec::new()); break; }
                Ok(n) => { let _ = send_bytes.send(buf[..n].to_vec()); }
                Err(_) => { let _ = send_bytes.send(Vec::new()); break; }
            }
        }
    });

    // Hook events (UDS) -> WS via channel
    let (send_ev, recv_ev) = crossbeam_channel::unbounded::<String>();
    let _hooks_thread = std::thread::spawn({
        let uds = uds;
        move || {
            let mut buf = [0u8; 8192];
            loop {
                match uds.recv(&mut buf) {
                    Ok(n) => {
                        if n == 0 { continue; }
                        let s = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = send_ev.send(s);
                    }
                    Err(_) => break,
                }
            }
        }
    });

    // Shared state: which block is currently open
    let current_block: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let current_block_for_events = current_block.clone();

    // Task A: forward PTY bytes + synthesize alt-screen + block_chunk
    let ws_tx_forward = ws_tx_arc.clone();
    let forward_task = tokio::spawn(async move {
        let mut alt_screen = false;
        for chunk in recv_bytes.iter() {
            if chunk.is_empty() { let _ = ws_tx_forward.lock().await.send(Message::Close(None)).await; break; }

            // Always stream raw bytes to xterm
            if ws_tx_forward.lock().await.send(Message::Binary(chunk.clone())).await.is_err() { break; }

            // Alt-screen detect
            if !alt_screen && contains_seq(&chunk, b"\x1b[?1049h") {
                alt_screen = true;
                let _ = ws_tx_forward.lock().await.send(Message::Text(r#"{"type":"alt_screen","on":true}"#.into())).await;
            }
            if alt_screen && contains_seq(&chunk, b"\x1b[?1049l") {
                alt_screen = false;
                let _ = ws_tx_forward.lock().await.send(Message::Text(r#"{"type":"alt_screen","on":false}"#.into())).await;
            }

            // If a block is open, also send plaintext to the timeline as block_chunk
            if let Some(id) = &*current_block.lock().await {
                let text = String::from_utf8_lossy(&chunk).to_string();
                let msg = serde_json::json!({"type":"block_chunk","id": id, "text": text});
                if ws_tx_forward.lock().await.send(Message::Text(msg.to_string())).await.is_err() { break; }
            }
        }
    });

    // Task B: forward hook events; update current_block
    let ws_tx_events = ws_tx_arc.clone();
    let events_task = tokio::spawn(async move {
        for s in recv_ev.iter() {
            // Try to parse just enough to update state
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if v.get("type").and_then(|x| x.as_str()) == Some("block_event") {
                    match v.get("event").and_then(|x| x.as_str()) {
                        Some("opened") => {
                            if let Some(id) = v.get("block").and_then(|b| b.get("id")).and_then(|x| x.as_str()) {
                                *current_block_for_events.lock().await = Some(id.to_string());
                            }
                        }
                        Some("closed") => {
                            *current_block_for_events.lock().await = None;
                        }
                        _ => {}
                    }
                }
            }
            // Forward raw event to client
            let _ = ws_tx_events.lock().await.send(Message::Text(s)).await;
        }
    });

    // Task C: WS -> PTY (keystrokes or control JSON like resize)
    let writer2 = writer.clone();
    let master2 = master_arc.clone();
    let input_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Binary(b) => {
                    let mut w = writer2.lock().await;
                    let _ = w.write_all(&b);
                    let _ = w.flush();
                }
                Message::Text(t) => {
                    if t.starts_with('{') {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                            if v.get("type").and_then(|x| x.as_str()) == Some("resize") {
                                if let (Some(cols), Some(rows)) = (
                                    v.get("cols").and_then(|x| x.as_u64()),
                                    v.get("rows").and_then(|x| x.as_u64()),
                                ) {
                                    let m = master2.lock().await;
                                    let _ = m.resize(PtySize {
                                        rows: rows as u16, cols: cols as u16,
                                        pixel_width: 0, pixel_height: 0
                                    });
                                    continue;
                                }
                            }
                        }
                    }
                    let mut w = writer2.lock().await;
                    let _ = w.write_all(t.as_bytes());
                    let _ = w.flush();
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let _ = forward_task.await;
    let _ = events_task.await;
    let _ = input_task.await;
    let _ = child.kill();
    let _ = fs::remove_file(&sock_path);
    
    // Clean up session
    session_manager.remove_session(&session_id).await;
    eprintln!("[{}] WebSocket disconnected, session cleaned up", session_id);
}

// Helpers
fn make_sock_path() -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let pid = std::process::id();
    PathBuf::from(format!("/tmp/bridge-{}-{}.sock", pid, ts))
}
fn contains_seq(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}