#!/usr/bin/env bash
# Boots the server locally and checks the ticket 06 HTTP-layer behavior:
# response compression on GET /catalog, and the CORS allowlist. Prints
# uncompressed vs. brotli-compressed byte size, and confirms an allowed
# Origin gets Access-Control-Allow-Origin while a disallowed one doesn't.
#
# Requires apps/server/.env with DATABASE_URL and CORS_ALLOWED_ORIGINS set
# (see apps/server/.env.example) and a seeded Postgres to compare against.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SERVER_DIR="$WORKSPACE_DIR/apps/server"

HOST="${HOST:-localhost}"
PORT="${PORT:-3000}"
BASE_URL="http://$HOST:$PORT/api/v1"
ALLOWED_ORIGIN="${ALLOWED_ORIGIN:-http://localhost:1420}"
DISALLOWED_ORIGIN="http://evil.example"

LOG_FILE="$(mktemp)"
echo ">> building server"
(cd "$SERVER_DIR" && cargo build --bin server)

echo ">> starting server (log: $LOG_FILE)"
(cd "$SERVER_DIR" && exec "$WORKSPACE_DIR/target/debug/server") >"$LOG_FILE" 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT

echo ">> waiting for server to come up"
for _ in $(seq 1 30); do
  curl -s -o /dev/null "$BASE_URL/healthcheck" && break
  sleep 1
done

echo "=== /catalog uncompressed size ==="
curl -s -o /dev/null -w "%{size_download} bytes\n" "$BASE_URL/catalog"

echo "=== /catalog compressed (br) ==="
curl -s -o /dev/null -D - -w "wire_bytes=%{size_download}\n" -H "Accept-Encoding: br" "$BASE_URL/catalog" \
  | grep -i "content-encoding\|wire_bytes"

echo "=== CORS: allowed origin ($ALLOWED_ORIGIN) ==="
curl -s -o /dev/null -D - -H "Origin: $ALLOWED_ORIGIN" "$BASE_URL/healthcheck" \
  | grep -i "access-control-allow-origin" || echo "MISSING — check CORS_ALLOWED_ORIGINS in .env"

echo "=== CORS: disallowed origin ($DISALLOWED_ORIGIN) ==="
if curl -s -o /dev/null -D - -H "Origin: $DISALLOWED_ORIGIN" "$BASE_URL/healthcheck" \
  | grep -qi "access-control-allow-origin"; then
  echo "UNEXPECTED — should have been rejected"
else
  echo "correctly rejected (no ACAO header)"
fi
