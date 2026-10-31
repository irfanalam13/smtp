#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

worker_enabled=false
if [[ "${1:-}" == "--worker" ]]; then
  worker_enabled=true
elif [[ "${1:-}" == "--no-worker" ]]; then
  worker_enabled=false
elif [[ $# -gt 0 ]]; then
  printf 'Usage: %s [--worker|--no-worker]\n' "$0" >&2
  exit 2
fi

for command in docker cargo go pnpm; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'Missing required command: %s\n' "$command" >&2
    exit 1
  fi
done

if ! docker compose version >/dev/null 2>&1; then
  printf 'Docker Compose v2 is required.\n' >&2
  exit 1
fi

if [[ ! -f .env ]]; then
  printf 'Missing .env. Create it with: cp .env.example .env\n' >&2
  exit 1
fi

set -a
# shellcheck disable=SC1091
source .env
set +a

: "${POSTGRES_PASSWORD:?Set POSTGRES_PASSWORD in .env}"
: "${POSTGRES_HOST_PORT:=5432}"
: "${DATABASE_URL:=postgres://smtp_dev:${POSTGRES_PASSWORD}@localhost:${POSTGRES_HOST_PORT}/smtp_platform?sslmode=disable}"
: "${API_ADDR:=127.0.0.1:8080}"
: "${MAILBOX_SERVICE_ADDR:=127.0.0.1:8081}"
: "${MAILBOX_SERVICE_URL:=http://127.0.0.1:8081}"
: "${CONTROL_PLANE_DEV_MODE:=true}"
APP_ENVIRONMENT=development
case "$API_ADDR" in
  127.0.0.1:*) ;;
  :*)
    printf 'Rebinding wildcard API address %s to localhost for local development.\n' "$API_ADDR" >&2
    API_ADDR="127.0.0.1${API_ADDR}"
    ;;
  *)
    printf 'API_ADDR must use 127.0.0.1 in the localhost-only development runner (got %s).\n' "$API_ADDR" >&2
    exit 1
    ;;
esac
if [[ ! "${MAILBOX_SERVICE_HMAC_KEY:-}" =~ ^[[:xdigit:]]{64,}$ ]]; then
  printf 'MAILBOX_SERVICE_HMAC_KEY must be at least 64 hexadecimal characters in .env.\n' >&2
  exit 1
fi
export DATABASE_URL POSTGRES_HOST_PORT API_ADDR APP_ENVIRONMENT CONTROL_PLANE_DEV_MODE
export MAILBOX_SERVICE_ADDR MAILBOX_SERVICE_URL MAILBOX_SERVICE_HMAC_KEY
if [[ "$worker_enabled" == true && "${SMTP_DELIVERY_ENABLED:-false}" != "true" ]]; then
  printf 'The outbound worker requires SMTP_DELIVERY_ENABLED=true in .env.\n' >&2
  exit 1
fi
if [[ "$worker_enabled" == true && -z "${SMTP_RELAY_HOST:-}" && -z "${SMTP_DELIVERY_ALLOWLIST:-}" ]]; then
  printf 'The outbound worker requires SMTP_DELIVERY_ALLOWLIST in .env.\n' >&2
  printf 'For Gmail/Brevo delivery, configure SMTP_RELAY_* settings instead.\n' >&2
  exit 1
fi
if [[ "$worker_enabled" == true &&
  ( -n "${SMTP_RELAY_HOST:-}" || -n "${SMTP_RELAY_PORT:-}" ||
    -n "${SMTP_RELAY_USERNAME:-}" || -n "${SMTP_RELAY_PASSWORD:-}" ||
    -n "${SMTP_RELAY_FROM:-}" ) ]]; then
  if [[ -z "${SMTP_RELAY_PORT:-}" || -z "${SMTP_RELAY_USERNAME:-}" ||
    -z "${SMTP_RELAY_PASSWORD:-}" || -z "${SMTP_RELAY_FROM:-}" ]]; then
    printf 'SMTP relay delivery requires host, port, username, password, and From settings in .env.\n' >&2
    exit 1
  fi
fi

children=()
cleanup() {
  local child
  for child in "${children[@]}"; do
    kill "$child" 2>/dev/null || true
  done
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

docker compose up -d postgres

printf 'Waiting for PostgreSQL...\n'
for attempt in {1..30}; do
  if docker compose exec -T postgres pg_isready -U smtp_dev -d smtp_platform >/dev/null 2>&1; then
    break
  fi
  if [[ "$attempt" -eq 30 ]]; then
    printf 'PostgreSQL did not become ready.\n' >&2
    exit 1
  fi
  sleep 1
done

start_process() {
  local name="$1"
  shift
  printf 'Starting %s...\n' "$name"
  "$@" &
  children+=("$!")
}

start_process "API" go run ./smtp-api/cmd/api
start_process "mailbox service" cargo run -p mailbox-service
start_process "dashboard" pnpm dev:dashboard
start_process "SMTP ingress" cargo run -p smtp-ingress
if [[ "$worker_enabled" == true ]]; then
  start_process "outbound delivery worker" cargo run -p mail-delivery
fi

cat <<'EOF'

Development services are running:
  API:       http://localhost:8080
  Dashboard: http://localhost:3000
  SMTP:      127.0.0.1:2525
EOF
printf '  Database:  localhost:%s\n\n' "$POSTGRES_HOST_PORT"
cat <<'EOF'

Use Ctrl-C to stop application processes. PostgreSQL remains running for the
next start; use `make dev-down` when you want to stop it.
EOF

status=0
wait -n "${children[@]}" || status=$?
printf 'A development process exited with status %d; stopping the remaining processes.\n' "$status" >&2
exit "$status"
