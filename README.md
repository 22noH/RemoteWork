**English** | [한국어](README.ko.md)

# ERemote

A TeamViewer-style, source-available remote desktop application. A hybrid of a
Rust host and a TypeScript viewer, delivering real-time screen sharing and
remote control over a WebRTC P2P connection.

> **License:** [Business Source License 1.1](LICENSE) — **free for non-commercial
> use**; commercial/production use requires a separate commercial license.
> Converts to Apache 2.0 on 2030-07-17. See [License](#license) below.

## Components

| Component | Description |
|-----------|-------------|
| `signaling-server/` | Rust WebSocket signaling server (TLS required) |
| `host-agent/` | Rust host agent — screen capture, WebRTC, input injection, audio, host GUI (egui) |
| `viewer-client/` | TypeScript + React + Electron viewer |
| `proto/` | Protobuf message definitions |
| `deploy/` | Self-hosting deployment package (nginx + TLS + certbot, opt-in TURN) |

## Quick start

**For full setup and build instructions, see [`docs/04_SETUP_AND_BUILD.md`](docs/04_SETUP_AND_BUILD.md).**

```bash
# 1. Signaling server (--insecure is required for local dev — plaintext WS)
cd signaling-server && cargo run -- --insecure

# 2. Host agent (the PC whose screen is shared)
cd host-agent && cargo run

# 3. Viewer (the PC connecting remotely)
cd viewer-client && npm install && npm run dev

# Or run it as the Electron app
cd viewer-client && npm run electron:dev
```

> For production deployment (nginx + TLS + certbot, optional TURN), see
> [`deploy/README.md`](deploy/README.md).

## Features

- Real-time screen sharing (xcap → VP8 → WebRTC)
- Remote keyboard/mouse control
- Monitor selection (viewer picks which monitor to see; host reconfigures the capturer)
- Two-way audio (Opus)
- Chat and file transfer
- Host GUI (egui) — connect approve/deny prompt, view-only toggle, chat, file-receive approval, minimizes to the taskbar on close
- One-time password (regenerated on every launch, never written to disk)
- 1:1 connection (only one viewer at a time)
- View-only mode (`allow_control`) — when off, the viewer sees the screen but input is ignored
- Enforced TLS (the signaling server refuses to start without a TLS cert or `--insecure`)
- Argon2id password authentication
- Automatic reconnect (exponential backoff)
- Session idle timeout (5 minutes, with a warning)
- Electron desktop app packaging (Windows/macOS/Linux)
- Auto-updater (electron-updater)

## Documentation

| Document | Contents |
|----------|----------|
| [`docs/01_PROJECT_OVERVIEW.md`](docs/01_PROJECT_OVERVIEW.md) | Project structure and tech-stack detail |
| [`docs/04_SETUP_AND_BUILD.md`](docs/04_SETUP_AND_BUILD.md) | Development setup and build guide |
| [`deploy/README.md`](deploy/README.md) | Production deployment guide (nginx + TLS + certbot, optional TURN) |

> The docs are written in Korean.

## Tech stack

- **Signaling Server**: Rust, tokio, tungstenite, prost (Protobuf), serde_json, argon2
- **Host Agent**: Rust, xcap, vpx-encode, webrtc-rs 0.11, enigo, cpal, opus, eframe/egui 0.28, sys-locale
- **Viewer Client**: TypeScript, React 18, Vite, Electron, Zustand, WebRTC API, electron-updater

## License

[Business Source License 1.1](LICENSE) (BSL) — the source is available, but this
is not "open source" in the OSI sense.

| Use | Allowed |
|-----|---------|
| Personal / non-commercial / evaluation / development / internal testing | ✅ Free |
| Viewing / modifying / redistributing the source | ✅ Free |
| **Commercial / production use** (in a product or service, offering to third parties, etc.) | ⚠️ **Commercial license required** (inquiries: [github.com/22noH/ERemote](https://github.com/22noH/ERemote)) |

- After the **Change Date (2030-07-17)**, each version automatically converts to
  the **Apache License 2.0**.
- The code is public, so **self-hosting** is allowed within the terms above, but
  commercial use requires a license.
