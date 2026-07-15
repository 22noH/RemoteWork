#!/usr/bin/env sh
# One-time Let's Encrypt certificate bootstrap. Run this once on the server
# (after DNS for $DOMAIN points at it and ports 80/443 are open), then use
# `docker compose up -d` normally afterwards.
set -eu
cd "$(dirname "$0")"

[ -f .env ] || { echo "Create deploy/.env first (copy .env.example)"; exit 1; }
. ./.env
: "${DOMAIN:?set DOMAIN in deploy/.env}"
: "${LETSENCRYPT_EMAIL:?set LETSENCRYPT_EMAIL in deploy/.env}"

live="./certbot/conf/live/$DOMAIN"
mkdir -p "$live" ./certbot/www

echo "### 1/4  temporary self-signed cert so nginx can start"
docker compose run --rm --entrypoint openssl certbot \
  req -x509 -nodes -newkey rsa:2048 -days 1 \
  -keyout "/etc/letsencrypt/live/$DOMAIN/privkey.pem" \
  -out    "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" \
  -subj "/CN=$DOMAIN"

echo "### 2/4  starting web + signaling"
docker compose up -d --build web signaling

echo "### 3/4  requesting the real certificate from Let's Encrypt"
rm -rf "$live"
docker compose run --rm --entrypoint certbot certbot \
  certonly --webroot -w /var/www/certbot \
  --email "$LETSENCRYPT_EMAIL" --agree-tos --no-eff-email \
  -d "$DOMAIN"

echo "### 4/4  reloading nginx with the real certificate"
docker compose exec web nginx -s reload

echo
echo "Done — https://$DOMAIN should be live."
echo "Start everything (incl. the renewal loop) with:  docker compose up -d"
