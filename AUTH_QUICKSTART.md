# 🔒 Authentication Quickstart

## Overview

Bridge for Warp Terminal now supports **optional token-based authentication** with QR code support for easy mobile access!

## Features

- 🔐 **Token Authentication**: Secure your terminal with a shared secret
- 📱 **QR Code**: Scan to connect from mobile instantly
- 💾 **Session Persistence**: Token stored securely in browser session
- 🔄 **Auto-Reconnect**: Handles network interruptions gracefully
- ⏳ **Session Timeout**: Auto-cleanup idle sessions (default: 30 min)
- 🔙 **Backwards Compatible**: Auth is optional, disabled by default

## Quick Setup

### 1. Generate a Token

```bash
# Generate a secure 32-byte hex token
openssl rand -hex 32
```

**Example output**: `a7b3c9d2e1f4g5h6i7j8k9l0m1n2o3p4q5r6s7t8u9v0w1x2y3z4a5b6c7d8e9f0`

### 2. Set Environment Variable

```bash
export BRIDGE_AUTH_TOKEN=your-token-here
```

Or create a `.env` file:

```bash
# .env file
BRIDGE_AUTH_TOKEN=a7b3c9d2e1f4g5h6i7j8k9l0m1n2o3p4q5r6s7t8u9v0w1x2y3z4a5b6c7d8e9f0
BRIDGE_SESSION_TIMEOUT=1800  # 30 minutes (optional)
```

### 3. Start the Server

```bash
cd bridge-ws-stub
cargo run
```

You'll see:

```
🔒 Bridge listening at http://0.0.0.0:7777  (Authentication ENABLED)
   📱 QR Code available at http://0.0.0.0:7777/qr
   WebSocket: ws://0.0.0.0:7777/bridge
   Static files: ./static
```

## Usage

### Desktop Browser

1. Visit `http://localhost:7777`
2. Enter your token in the modal
3. Click "Connect"
4. You're in! 🎉

### Mobile Device (QR Code Flow)

1. **On Desktop**: Visit `http://localhost:7777/qr`
2. **On Mobile**: 
   - Open Camera app
   - Point at QR code
   - Tap the notification
   - Browser opens → Automatically connected! 🚀

The QR code contains: `ws://your-host:7777/bridge?token=YOUR_TOKEN`

### Tailscale Setup

Perfect for your use case! Both devices on Tailscale VPN:

1. Set auth token on the Mac running Bridge
2. Desktop: Use `http://localhost:7777` or Tailscale IP
3. Mobile: Scan QR code with Tailscale IP embedded
4. Both authenticated securely over encrypted VPN

## Token Storage

**Where tokens are stored**:
- ✅ `sessionStorage` - Cleared when tab closes (secure)
- ❌ NOT in `localStorage` - Would persist across sessions (less secure)
- ❌ NOT in cookies - Avoid CSRF risks

**Token lifecycle**:
1. Enter token → Stored in `sessionStorage`
2. Page reload → Token retrieved from `sessionStorage`
3. Close tab → Token automatically cleared
4. QR scan → Token extracted from URL → Stored → URL cleaned

## Configuration Options

```bash
# Required (if you want auth)
BRIDGE_AUTH_TOKEN=your-token-here

# Optional session timeout in seconds
BRIDGE_SESSION_TIMEOUT=1800  # Default: 30 minutes

# Server settings (existing)
BRIDGE_HOST=0.0.0.0
BRIDGE_PORT=7777
BRIDGE_SHELL=/bin/zsh
```

## API Endpoints

### WebSocket Connection
```
ws://host:port/bridge?token=YOUR_TOKEN
```

### QR Code Page
```
GET /qr
```
Returns beautiful HTML page with QR code and instructions.

### Active Sessions
```
GET /sessions
```
Returns JSON with active session count and details:
```json
{
  "active_sessions": 2,
  "sessions": [
    {
      "id": "uuid-here",
      "created_at": "2025-11-28T06:00:00Z",
      "last_activity": "2025-11-28T06:15:00Z",
      "device_info": null,
      "user_agent": null
    }
  ]
}
```

## Security Notes

### For Tailscale Users (Your Setup)

✅ **What You Have**:
- Network-level encryption (Tailscale VPN)
- Private mesh network (no public internet)
- Device authentication (Tailscale ACLs)

✅ **What Bridge Adds**:
- Application-level authentication
- Session management
- Prevents unauthorized Tailscale devices from accessing terminal

### For Public/Production Deployments

⚠️ **Additional Requirements**:
1. **HTTPS/WSS**: Use reverse proxy with TLS
2. **Strong Tokens**: 32+ bytes, cryptographically random
3. **Rate Limiting**: Prevent brute force attacks
4. **IP Whitelisting**: Restrict to known IPs
5. **Audit Logging**: Track all commands (Phase 1.3)

## Troubleshooting

### "Authentication Required" Modal Won't Go Away
- Check token is correct
- Verify `BRIDGE_AUTH_TOKEN` environment variable is set
- Check server logs for auth errors
- Clear `sessionStorage` and try again: DevTools → Application → Session Storage

### QR Code Shows "Authentication not enabled"
- Server doesn't have `BRIDGE_AUTH_TOKEN` set
- Set the environment variable and restart server

### Connection Keeps Dropping
- Check session timeout setting (default 30 min)
- Mobile device may be sleeping/backgrounded
- Network issues - check Tailscale connection

### Can't Connect from Mobile
- Ensure both devices on same VPN/network
- Use Tailscale IP in QR code, not `localhost`
- Check firewall rules on Mac

## Disabling Authentication

Want to go back to no-auth mode?

```bash
# Just don't set the token
unset BRIDGE_AUTH_TOKEN

# Or comment out in .env
# BRIDGE_AUTH_TOKEN=...
```

Server will show:
```
⚠️  Bridge listening at http://0.0.0.0:7777  (Authentication DISABLED - Set BRIDGE_AUTH_TOKEN to enable)
```

## Testing Checklist

- [ ] Server starts with auth enabled
- [ ] Desktop browser shows auth modal
- [ ] Valid token connects successfully
- [ ] Invalid token shows error
- [ ] QR page loads at `/qr`
- [ ] Mobile camera scans QR code
- [ ] Mobile browser auto-connects
- [ ] Token persists across page reload
- [ ] Token cleared when tab closes
- [ ] Sessions endpoint returns data
- [ ] Idle sessions cleanup after timeout

## Next Steps

**Phase 1.2**: Enhanced Session Management
- Device info capture
- Session resume after disconnect
- Admin dashboard

**Phase 1.3**: Audit Logging
- Command history
- Security audit trail
- Log rotation

---

**Questions?** Check `PROGRESS.md` for implementation details or `WARP.md` for architecture overview.
