use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{routing::get, Router};
use futures::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixDatagram;
use std::os::unix::prelude::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/bridge", get(ws_handler))
        .nest_service("/", ServeDir::new("static"));

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 7777));
    println!("Bridge listening at http://{}  (WS:/bridge, static: ./static)", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
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
    let mut master = pair.master;
    let mut reader = match master.try_clone_reader() {
        Ok(r) => r, Err(e) => { eprintln!("clone reader: {e}"); let _=fs::remove_file(&sock_path); return; }
    };
    let writer = match master.take_writer() {
        Ok(w) => w, Err(e) => { eprintln!("take writer: {e}"); let _=fs::remove_file(&sock_path); return; }
    };
    let writer = Arc::new(Mutex::new(writer));
    let master_arc = Arc::new(Mutex::new(master));

    let (mut ws_tx, mut ws_rx) = socket.split();
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
                                    let mut m = master2.lock().await;
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