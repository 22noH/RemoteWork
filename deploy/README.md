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
(corporate firewalls, some mobile carriers). The docker-compose relay below
listens on `3478`/`5349` — fine for many networks. But the strictest firewalls
allow **only `80`/`443`** and block everything else (including `3478`/`5349`);
for those, run TURN on port **443** instead — see
[Strict networks (only 80/443 open)](#strict-networks-only-80443-open).

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

## Strict networks (only 80/443 open)

Some networks — locked-down corporate LANs especially — block **everything
except TCP 80/443** outbound. There `3478`/`5349` are dead, so TURN must listen
on **443**. The all-in-one VPS above already serves the web app on 443, so TURN
can't share it there — give TURN its **own** small server (its own public IP),
running coturn standalone (not the docker-compose relay).

On that separate box (Ubuntu example):

```sh
sudo apt-get install -y coturn libcap2-bin

sudo tee /etc/turnserver.conf >/dev/null <<'CONF'
listening-port=443
lt-cred-mech
user=eremote:CHANGE-ME-to-a-long-random-string
realm=eremote
# On NAT'd clouds (Oracle/AWS/GCP) the VM only sees a private IP — advertise both:
external-ip=PUBLIC_IP/PRIVATE_IP        # e.g. 203.0.113.5/10.0.0.87
min-port=49152
max-port=65535
fingerprint
no-cli
no-tls
no-dtls
no-tcp-relay
CONF

sudo sed -i 's/^#*TURNSERVER_ENABLED=.*/TURNSERVER_ENABLED=1/' /etc/default/coturn
sudo setcap cap_net_bind_service=+ep /usr/bin/turnserver   # let it bind privileged :443
sudo systemctl enable --now coturn
```

Open **TCP 443**, **UDP 443**, and **UDP 49152-65535** in **both** firewalls:

- the **cloud** firewall (AWS security group / Oracle security list / GCP firewall), and
- the **host** firewall (`iptables`/`ufw`) — cloud Ubuntu images often ship an
  iptables ruleset that rejects everything except port 22.

On NAT'd clouds also confirm the subnet's route table sends `0.0.0.0/0` to an
internet gateway, or nothing is reachable at all.

Point the host/viewer at it (note `?transport=tcp` — that's what rides 443):

```
TURN_URL=turn:PUBLIC_IP:443?transport=tcp
TURN_USERNAME=eremote
TURN_CREDENTIAL=<password>
```

The viewer takes the same URL under **Advanced Settings → TURN Server URL**.

### If plain TCP/443 is still blocked (deep packet inspection)

A few firewalls inspect payloads and drop non-TLS traffic on 443. Then wrap TURN
in TLS: get a domain (a free `duckdns.org` subdomain works) with an `A` record to
the server, obtain a Let's Encrypt cert (`certbot certonly --standalone -d DOMAIN`
— stop coturn first so certbot can use 443), then in `turnserver.conf` drop
`no-tls`/`no-dtls` and add:

```
tls-listening-port=443
cert=/etc/letsencrypt/live/DOMAIN/fullchain.pem
pkey=/etc/letsencrypt/live/DOMAIN/privkey.pem
```

Clients then use `turns:DOMAIN:443` (note the **s**). The cert is free; only a
domain is needed, and free subdomains work.

## Notes

- TLS is terminated at nginx; the signaling server runs `--insecure` but is only
  reachable on the internal Docker network, never the public internet.
- Certificates auto-renew (the `certbot` service checks every 12h).
- The viewer is domain-agnostic: the same build works on any domain because it
  derives the signaling URL from the page's own address at runtime.
