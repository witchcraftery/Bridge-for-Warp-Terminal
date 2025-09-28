# 🌉 Bridge for Warp Terminal

> A secure, browser-based remote gateway to your local Warp Terminal, enabling powerful terminal access from anywhere.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![WebSocket](https://img.shields.io/badge/WebSocket-010101?style=for-the-badge&logo=socket.io&logoColor=white)](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
[![Terminal](https://img.shields.io/badge/Terminal-4D4D4D?style=for-the-badge&logo=windows-terminal&logoColor=white)](https://www.warp.dev/)

Bridge for Warp Terminal provides a seamless connection between your browser and your local Warp Terminal instance, allowing you to harness the full power of Warp's AI-enhanced terminal experience from any device with a web browser.

## 📋 Table of Contents

- [Features](#-features)
- [Architecture](#-architecture)
- [Prerequisites](#-prerequisites)
- [Installation](#-installation)
- [Configuration](#-configuration)
- [Usage](#-usage)
- [API Documentation](#-api-documentation)
- [Security](#-security)
- [Development](#-development)
- [Troubleshooting](#-troubleshooting)
- [Contributing](#-contributing)
- [License](#-license)

## ✨ Features

- **🌐 Browser-Based Access**: Access your Warp Terminal from any modern web browser
- **🔌 Real-time WebSocket Connection**: Low-latency, bidirectional communication between browser and terminal
- **🎨 Dual Interface Modes**:
  - **Agent View**: Track command execution history with organized timeline cards
  - **Live Terminal**: Full terminal emulator with xterm.js for interactive sessions
- **📱 Responsive Design**: Optimized for desktop, tablet, and mobile devices
- **⌨️ Virtual Keyboard Support**: Touch-friendly controls for mobile devices
- **🔄 Session Management**: Automatic session handling with PTY (pseudo-terminal) support
- **🎯 Command Context Tracking**: Captures working directory, command, and exit status
- **🚀 Alt-Screen Detection**: Seamlessly switches between normal and alternate screen buffers (for vim, less, etc.)
- **📦 Lightweight**: Minimal dependencies with efficient Rust backend
- **🛡️ Secure by Design**: Unix domain sockets for inter-process communication

## 🏗 Architecture

Bridge for Warp Terminal consists of three main components:

```
┌─────────────────┐     WebSocket      ┌──────────────┐      PTY       ┌───────────────┐
│                 │◄──────────────────►│              │◄───────────────►│               │
│   Web Browser   │                    │  Bridge Server│                 │ Warp Terminal │
│   (index.html)  │     Messages       │   (Rust)     │     Commands    │    (Shell)    │
│                 │◄──────────────────►│              │◄───────────────►│               │
└─────────────────┘                    └──────────────┘                 └───────────────┘
        │                                      │                                │
        │                                      │                                │
        ▼                                      ▼                                ▼
   [xterm.js UI]                        [Axum WebServer]                  [Zsh/Bash/Fish]
   [Timeline View]                      [PTY Management]                  [Shell Hooks]
   [Touch Controls]                     [Session Handler]                 [Command Exec]
```

### Key Components:

1. **Web Frontend** (`static/index.html`):
   - xterm.js terminal emulator
   - Timeline view for command history
   - WebSocket client for real-time communication
   - Responsive UI with touch controls

2. **Rust Backend** (`src/main.rs`):
   - Axum web framework for HTTP/WebSocket handling
   - PTY (pseudo-terminal) management with portable-pty
   - Unix domain socket communication for shell hooks
   - Async I/O with Tokio runtime

3. **Shell Integration**:
   - Environment variable injection (`BRIDGE_SOCK`)
   - Support for Zsh hooks and other shell integrations
   - Command tracking and metadata collection

## 📋 Prerequisites

- **Rust**: Version 1.70.0 or higher
- **Cargo**: Rust's package manager (comes with Rust)
- **Operating System**: macOS, Linux, or WSL2 on Windows
- **Shell**: Zsh, Bash, or Fish (Zsh recommended for best integration)
- **Browser**: Any modern browser with WebSocket support

## 🚀 Installation

### 1. Clone the Repository

```bash
git clone https://github.com/witchcraftery/bridge-for-warp.git
cd bridge-for-warp
```

### 2. Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Build the Project

```bash
cd bridge-ws-stub
cargo build --release
```

The compiled binary will be available at `target/release/bridge-ws-stub`.

### 4. Quick Test

```bash
cargo run
```

Navigate to `http://localhost:7777` in your browser to test the connection.

## ⚙️ Configuration

### Environment Variables

Create a `.env` file in the project root (see `.env.example` for reference):

```bash
# Server Configuration
BRIDGE_HOST=0.0.0.0
BRIDGE_PORT=7777

# Shell Configuration
BRIDGE_SHELL=/bin/zsh  # or /bin/bash, /bin/fish

# Security (for production)
BRIDGE_AUTH_TOKEN=your-secure-token-here
BRIDGE_ALLOWED_ORIGINS=https://your-domain.com

# Logging
RUST_LOG=info  # debug, info, warn, error
```

### Shell Integration (Optional)

For enhanced functionality with Zsh, add to your `~/.zshrc`:

```bash
# Bridge for Warp Terminal integration
if [ -n "$BRIDGE_SOCK" ]; then
    # Add your custom shell hooks here
    # Example: Send command info to Bridge
    precmd() {
        echo "{\"type\":\"precmd\",\"pwd\":\"$PWD\"}" | nc -U "$BRIDGE_SOCK" 2>/dev/null
    }
fi
```

## 🎯 Usage

### Starting the Server

#### Development Mode
```bash
cd bridge-ws-stub
cargo run
```

#### Production Mode
```bash
cd bridge-ws-stub
cargo build --release
./target/release/bridge-ws-stub
```

#### With Custom Configuration
```bash
BRIDGE_PORT=8080 BRIDGE_SHELL=/bin/bash ./target/release/bridge-ws-stub
```

### Accessing the Interface

1. Open your browser and navigate to:
   - Local: `http://localhost:7777`
   - Network: `http://your-ip:7777`

2. The interface offers two modes:
   - **Agent View**: See command history and outputs in timeline cards
   - **Live Terminal**: Direct terminal interaction

### Using the Interface

#### Agent View (Default)
- Type commands in the composer at the bottom
- Press Enter to execute
- View results in timeline cards
- Click "Expand" to see full output
- Click "Copy" to copy command output

#### Live Terminal Mode
- Click "Live Terminal" tab
- Toggle "Live typing: ON" for direct input
- Use virtual keyboard for special keys
- Supports full terminal applications (vim, nano, etc.)

### Keyboard Shortcuts

| Shortcut | Action |
|----------|---------|
| `Enter` | Execute command (Agent mode) |
| `Shift+Enter` | New line in composer |
| `Ctrl+C` | Interrupt current process |
| `Tab` | Auto-completion |
| `Esc` | Exit insert mode (vim) |

## 📡 API Documentation

### WebSocket Protocol

Connect to: `ws://localhost:7777/bridge`

#### Message Types

##### Client → Server

**Text Input**:
```javascript
// Send keystrokes/commands
websocket.send(new TextEncoder().encode("ls -la\n"))
```

**Control Commands**:
```json
{
  "type": "resize",
  "cols": 80,
  "rows": 24
}
```

##### Server → Client

**Binary Data** (Terminal Output):
```javascript
// Raw terminal output bytes
websocket.onmessage = (event) => {
  if (event.data instanceof ArrayBuffer) {
    // Process terminal output
  }
}
```

**Event Messages**:
```json
{
  "type": "block_event",
  "event": "opened|closed",
  "block": {
    "id": "unique-id",
    "cmd": "ls -la",
    "cwd": "/home/user",
    "exit": 0
  }
}
```

**Alt-Screen Notifications**:
```json
{
  "type": "alt_screen",
  "on": true|false
}
```

### HTTP Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | Serve static web interface |
| `/bridge` | WebSocket | WebSocket upgrade endpoint |

## 🔒 Security

### ⚠️ Important Security Considerations

1. **Authentication**: Currently, Bridge does not implement authentication. For production use:
   - Implement token-based authentication
   - Use HTTPS/WSS for encrypted connections
   - Add rate limiting to prevent abuse

2. **Network Exposure**:
   - By default, the server binds to `0.0.0.0:7777` (all interfaces)
   - For local-only access, bind to `127.0.0.1`
   - Use a reverse proxy (nginx, Caddy) for production deployment

3. **Command Execution**:
   - All commands are executed with the privileges of the user running Bridge
   - Consider implementing command filtering or sandboxing
   - Log all executed commands for audit purposes

4. **Session Security**:
   - Each WebSocket connection spawns a new shell session
   - Sessions are terminated when the connection closes
   - Implement session timeouts for idle connections

### Best Practices

1. **Use HTTPS in Production**:
   ```nginx
   server {
       listen 443 ssl http2;
       ssl_certificate /path/to/cert.pem;
       ssl_certificate_key /path/to/key.pem;
       
       location /bridge {
           proxy_pass http://localhost:7777;
           proxy_http_version 1.1;
           proxy_set_header Upgrade $http_upgrade;
           proxy_set_header Connection "upgrade";
       }
   }
   ```

2. **Implement Authentication**:
   - Add JWT token validation
   - Use OAuth2 for enterprise deployments
   - Implement API key authentication for simple setups

3. **Network Isolation**:
   - Use VPN for remote access
   - Implement IP whitelisting
   - Use SSH tunneling as an alternative

## 🛠 Development

### Project Structure

```
bridge-for-warp/
├── bridge-ws-stub/
│   ├── src/
│   │   └── main.rs          # Main server implementation
│   ├── static/
│   │   └── index.html       # Web interface
│   ├── Cargo.toml           # Rust dependencies
│   └── Cargo.lock           # Dependency lock file
├── .gitignore               # Git ignore rules
├── .env.example             # Example environment configuration
└── README.md                # This file
```

### Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Adding Features

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Run tests: `cargo test`
5. Commit: `git commit -am 'Add your feature'`
6. Push: `git push origin feature/your-feature`
7. Create a Pull Request

### Debugging

Enable debug logging:
```bash
RUST_LOG=debug cargo run
```

Monitor WebSocket traffic in browser:
```javascript
// In browser console
console.log(websocket.readyState);
websocket.addEventListener('message', console.log);
```

## 🔧 Troubleshooting

### Common Issues

#### Connection Refused
- **Issue**: Browser cannot connect to WebSocket
- **Solution**: 
  - Check if the server is running: `ps aux | grep bridge-ws-stub`
  - Verify port is not in use: `lsof -i :7777`
  - Check firewall settings

#### Terminal Not Responding
- **Issue**: Commands don't execute
- **Solution**:
  - Verify shell path: `which $SHELL`
  - Set correct shell: `BRIDGE_SHELL=/bin/bash cargo run`
  - Check PTY permissions

#### Garbled Output
- **Issue**: Special characters display incorrectly
- **Solution**:
  - Ensure UTF-8 locale: `export LANG=en_US.UTF-8`
  - Update terminal font size in UI
  - Clear browser cache

#### Permission Denied
- **Issue**: Cannot create Unix socket
- **Solution**:
  - Check `/tmp` permissions
  - Run with appropriate user permissions
  - Clean up stale sockets: `rm /tmp/bridge-*.sock`

#### High CPU Usage
- **Issue**: Server consuming excessive CPU
- **Solution**:
  - Check for infinite loops in shell config
  - Limit concurrent connections
  - Implement rate limiting

### Performance Tuning

```toml
# In Cargo.toml for optimized builds
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### FAQ

**Q: Can I use this with tmux/screen?**
A: Yes, Bridge works with terminal multiplexers. The alt-screen detection will automatically switch to Live Terminal mode.

**Q: Does it work on Windows?**
A: Yes, through WSL2. Native Windows support requires additional PTY handling.

**Q: Can multiple users connect simultaneously?**
A: Each WebSocket connection spawns an independent shell session. Multiple connections are supported but not recommended without proper authentication.

**Q: How do I customize the terminal appearance?**
A: Edit the xterm.js configuration in `static/index.html`. You can modify themes, fonts, and cursor styles.

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Code of Conduct

This project adheres to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

### Development Guidelines

- Follow Rust best practices and idioms
- Write tests for new features
- Update documentation for API changes
- Use conventional commits for clear history

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Warp Terminal](https://www.warp.dev/) for the inspiration
- [xterm.js](https://xtermjs.org/) for the terminal emulator
- [Axum](https://github.com/tokio-rs/axum) for the web framework
- [portable-pty](https://github.com/wez/portable-pty) for PTY handling

## 📞 Issues

For bugs and feature requests, please [open an issue](https://github.com/witchcraftery/bridge-for-warp/issues).

For security vulnerabilities, please email nick@witchcraftery.io instead of using the issue tracker.

---

<p align="center">Made with ❤️ by developers, for developers</p>
<p align="center">© 2024 Bridge for Warp Terminal. All rights reserved.</p>
