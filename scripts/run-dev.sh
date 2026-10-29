#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

worker_enabled=true
if [[ "${1:-}" == "--no-worker" ]]; then
  worker_enabled=false
elif [[ $# -gt 0 ]]; then
  printf 'Usage: %s [--no-worker]\n' "$0" >&2
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
: "${DATABASE_URL:=postgres://smtp_dev:${POSTGRES_PASSWORD}@localhost:5432/smtp_platform?sslmode=disable}"
: "${API_ADDR:=:8080}"
: "${CONTROL_PLANE_DEV_MODE:=true}"
export DATABASE_URL API_ADDR CONTROL_PLANE_DEV_MODE

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
start_process "dashboard" pnpm dev:dashboard
start_process "SMTP ingress" cargo run -p smtp-ingress
if [[ "$worker_enabled" == true ]]; then
  start_process "queue worker" cargo run -p mail-queue
fi

cat <<'EOF'

Development services are running:
  API:       http://localhost:8080
  Dashboard: http://localhost:3000
  SMTP:      127.0.0.1:2525
  Database:  localhost:5432

Use Ctrl-C to stop application processes. PostgreSQL remains running for the
next start; use `make dev-down` when you want to stop it.
EOF

status=0
wait -n "${children[@]}" || status=$?
printf 'A development process exited with status %d; stopping the remaining processes.\n' "$status" >&2
exit "$status"
