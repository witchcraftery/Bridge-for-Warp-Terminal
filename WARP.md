# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Bridge for Warp Terminal is a secure, browser-based remote gateway that provides terminal access through WebSocket connections. The project consists of a Rust backend (`bridge-ws-stub`) and a web frontend using xterm.js.

## Architecture

The system has three key components communicating via WebSocket and Unix domain sockets:

1. **Web Frontend** (`bridge-ws-stub/static/index.html`): Browser-based UI with xterm.js terminal emulator and timeline view for command history
2. **Rust Backend** (`bridge-ws-stub/src/main.rs`): Axum web server managing WebSocket connections, PTY (pseudo-terminal) sessions, and shell integration
3. **Shell Integration**: Environment variable injection (`BRIDGE_SOCK`) for shell hooks to capture command metadata

### Critical Data Flows

- **PTY → WebSocket**: Terminal output is read in a blocking thread via `portable-pty`, sent through a `crossbeam-channel`, then forwarded to WebSocket clients as binary messages
- **WebSocket → PTY**: Browser input (keystrokes or resize commands) flows to the PTY writer via async Tokio tasks
- **Shell Hooks → WebSocket**: Shell events arrive via Unix datagram socket (`BRIDGE_SOCK`), parsed as JSON, and used to track command blocks in timeline view
- **Alt-Screen Detection**: Escape sequences (`\x1b[?1049h/l`) are detected to toggle between timeline and live terminal modes automatically

## Common Development Commands

### Building and Running

```bash
# Development mode (with logging)
cd bridge-ws-stub
RUST_LOG=debug cargo run

# Production build
cd bridge-ws-stub
cargo build --release
./target/release/bridge-ws-stub

# Run with custom shell
BRIDGE_SHELL=/bin/bash cargo run

# Run with custom port
BRIDGE_PORT=8080 cargo run
```

### Testing and Quality

```bash
# Run tests
cd bridge-ws-stub
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Check for errors without building
cargo check
```

### Debugging

```bash
# Enable verbose logging
RUST_LOG=trace cargo run

# Monitor WebSocket connections
# In browser console:
# console.log(websocket.readyState);
# websocket.addEventListener('message', console.log);

# Check for stale sockets
ls -la /tmp/bridge-*.sock

# Clean up sockets
rm /tmp/bridge-*.sock
```

## Key Technical Details

### Concurrency Model

The backend uses a **hybrid async/sync approach**:
- **Tokio async runtime**: Handles WebSocket connections, HTTP serving, and message forwarding
- **Blocking threads**: Two dedicated threads per connection for PTY reading and Unix socket events (necessary because `portable-pty` readers are synchronous)
- **Shared state**: `Arc<Mutex<T>>` for PTY writer and WebSocket sink; used to coordinate between threads and async tasks

### Session Management

Each WebSocket connection spawns:
1. A new shell process with dedicated PTY
2. A unique Unix datagram socket at `/tmp/bridge-{pid}-{timestamp}.sock` with 0600 permissions
3. Three concurrent tasks: PTY→WS forwarding, Hook events forwarding, and WS→PTY input handling

Sessions are automatically cleaned up when:
- WebSocket connection closes
- Shell process exits
- Any critical task fails

### Message Protocol

**Client → Server:**
- Binary data: Raw keystrokes/terminal input
- JSON control messages: `{"type":"resize","cols":80,"rows":24}`

**Server → Client:**
- Binary data: Raw PTY output for xterm.js
- JSON events:
  - `{"type":"block_event","event":"opened|closed","block":{...}}`
  - `{"type":"alt_screen","on":true|false}`
  - `{"type":"block_chunk","id":"...","text":"..."}`

### Frontend State Management

The web UI operates in two modes:
- **Agent View (Timeline)**: Default mode showing command history as cards. Activated when shell hooks send block events
- **Live Terminal**: Full xterm.js mode. Automatically activated when alt-screen sequences are detected (vim, less, etc.)

## Configuration

Environment variables (see `.env.example`):

**Essential:**
- `BRIDGE_HOST` - Server bind address (default: `0.0.0.0`)
- `BRIDGE_PORT` - Server port (default: `7777`)
- `BRIDGE_SHELL` - Shell path (default: `/bin/zsh`)

**Security (production):**
- `BRIDGE_AUTH_TOKEN` - Authentication token (currently not implemented)
- `BRIDGE_ALLOWED_ORIGINS` - CORS origins
- `RUST_LOG` - Logging level (`error`, `warn`, `info`, `debug`, `trace`)

## Security Considerations

⚠️ **This project currently has NO authentication**. For production:

1. **Network Isolation**: Bind to `127.0.0.1` only or use VPN/SSH tunneling
2. **TLS Required**: Use reverse proxy (nginx/Caddy) with HTTPS/WSS
3. **Command Privileges**: All commands execute with the user's privileges - consider sandboxing
4. **Session Security**: Implement session timeouts and command logging for audit

## Development Guidelines

### Adding Features

1. **WebSocket Protocol Changes**: Update both `handle_socket()` in `main.rs` and message handlers in `index.html`
2. **New Event Types**: Add JSON schema handling in the events forwarding task (Task B)
3. **UI Enhancements**: Modify `index.html` - xterm.js config is in the `<script>` tag, styles are inline

### Code Style

- Follow Rust idioms: prefer `?` operator over `match` for error handling where appropriate
- Use `Arc<Mutex<T>>` sparingly; document when shared state is necessary
- Keep `main.rs` focused on connection handling; extract complex logic to separate modules for features beyond MVP

### Common Patterns

**Sending JSON events to client:**
```rust
let msg = serde_json::json!({"type": "my_event", "data": value});
ws_tx.lock().await.send(Message::Text(msg.to_string())).await
```

**Reading from crossbeam channel in async context:**
```rust
for item in channel_receiver.iter() {
    // Process item
}
```

## Dependencies

**Core Rust crates:**
- `tokio` - Async runtime with full features
- `axum` - Web framework for HTTP and WebSocket
- `portable-pty` - Cross-platform PTY management
- `crossbeam-channel` - Thread-safe channel for sync→async communication
- `tower-http` - Static file serving
- `serde_json` - JSON parsing

**Frontend:**
- `xterm.js` - Terminal emulator (loaded from CDN)

## Known Limitations

1. **No authentication/authorization** - All connections have full shell access
2. **No session persistence** - Refresh = new shell
3. **Single shell per connection** - No multiplexing/tabs
4. **Shell hook dependency** - Timeline mode requires shell integration (Zsh recommended)
5. **No command filtering** - All commands are executed as-is

## Troubleshooting

**Connection refused**: Check if server is running (`ps aux | grep bridge-ws-stub`) and port is available (`lsof -i :7777`)

**Terminal not responding**: Verify shell path with `which $SHELL` and set `BRIDGE_SHELL` if needed

**Garbled output**: Ensure UTF-8 locale (`export LANG=en_US.UTF-8`)

**Stale sockets**: Clean up with `rm /tmp/bridge-*.sock` if permissions issues occur

## URLs and Access

- Local development: `http://localhost:7777`
- WebSocket endpoint: `ws://localhost:7777/bridge`
- Static files served from: `bridge-ws-stub/static/`
