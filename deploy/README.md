# Deploying ERemote

One small Linux VPS runs everything: nginx (TLS + the viewer web app), the
signaling server, automatic Let's Encrypt certificates, and — optionally — a
coturn TURN relay.

```
                         ┌──────────────── your VPS ────────────────┐
  viewer (browser) ──────┤  nginx :443  ── /        → viewer (SPA)   │
  wss://DOMAIN/signal ───┤              ── /signal  → signaling :8080│
                         │  certbot (auto-renew)                     │
  host agent ───────────►┤  coturn :3478/5349  (only with --profile) │
  wss://DOMAIN/signal    └───────────────────────────────────────────┘
```

## Prerequisites

- A Linux server with **Docker** and the **docker compose** plugin.
- A **domain** with a DNS `A` record pointing at the server's public IP.
- Inbound **80** and **443** open. (For TURN also open `3478`, `5349`, and UDP
  `49152-65535`.)

## Deploy (signaling + viewer + TLS)

```sh
git clone https://github.com/22noH/ERemote.git
cd ERemote/deploy

cp .env.example .env
nano .env                     # set DOMAIN and LETSENCRYPT_EMAIL

sh init-letsencrypt.sh        # one-time: obtains the certificate
docker compose up -d          # start everything (incl. auto-renewal)
```

Open `https://DOMAIN` — that's the viewer. It automatically talks to
`wss://DOMAIN/signal`, so nothing else to configure on the web side.

Update later with:

```sh
git pull && docker compose up -d --build
```

## Point the host agent at your server

The host (the machine being controlled) connects to the same signaling URL:

```sh
SIGNALING_URL=wss://DOMAIN/signal ./host-agent        # Linux/macOS
```
```powershell
$env:SIGNALING_URL = "wss://DOMAIN/signal"; .\host-agent.exe   # Windows
```

(Or set `signaling_server_url` in the host's `config.json`.)

## TURN relay — turn it on only when you need it

Most home-to-home connections work without TURN. Enable it for strict networks
(corporate firewalls, some mobile carriers):

1. Edit `turnserver.conf` — replace `YOURDOMAIN` and the password.
2. Set the matching values in `.env` (`TURN_USER`, `TURN_PASSWORD`).
3. Start it: `docker compose --profile turn up -d`
4. Tell the host/viewer to use it:

   ```sh
   SIGNALING_URL=wss://DOMAIN/signal \
   TURN_URL=turns:DOMAIN:5349 \
   TURN_USERNAME=eremote \
   TURN_CREDENTIAL=<password> \
   ./host-agent
   ```

   The viewer takes the same values under **Advanced Settings** on the connect
   screen, or at build time via `VITE_TURN_URL` / etc.

Stop TURN again with `docker compose stop coturn`.

## Notes

- TLS is terminated at nginx; the signaling server runs `--insecure` but is only
  reachable on the internal Docker network, never the public internet.
- Certificates auto-renew (the `certbot` service checks every 12h).
- The viewer is domain-agnostic: the same build works on any domain because it
  derives the signaling URL from the page's own address at runtime.
