#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

for command in docker cargo go pnpm openssl; do
  command -v "$command" >/dev/null 2>&1 || {
    printf 'Missing required command: %s\n' "$command" >&2
    exit 1
  }
done

docker compose version >/dev/null 2>&1 || {
  printf 'Docker Compose v2 is required.\n' >&2
  exit 1
}

if [[ ! -f .env ]]; then
  cp .env.example .env
  password="$(openssl rand -hex 24)"
  sed -i "s/^POSTGRES_PASSWORD=.*/POSTGRES_PASSWORD=$password/" .env
  database_url="postgres://smtp_dev:${password}@localhost:5434/smtp_platform?sslmode=disable"
  sed -i "s|^DATABASE_URL=.*|DATABASE_URL=$database_url|" .env
  printf 'Created .env with a generated local PostgreSQL password.\n'
fi

mailbox_service_key="$(sed -n 's/^MAILBOX_SERVICE_HMAC_KEY=//p' .env | tail -n 1)"
if [[ ! "$mailbox_service_key" =~ ^[[:xdigit:]]{64,}$ ]]; then
  mailbox_service_key="$(openssl rand -hex 32)"
  if grep -q '^MAILBOX_SERVICE_HMAC_KEY=' .env; then
    sed -i "s/^MAILBOX_SERVICE_HMAC_KEY=.*/MAILBOX_SERVICE_HMAC_KEY=$mailbox_service_key/" .env
  else
    printf '\nMAILBOX_SERVICE_HMAC_KEY=%s\n' "$mailbox_service_key" >> .env
  fi
fi

set -a
# shellcheck disable=SC1091
source .env
set +a
: "${POSTGRES_PASSWORD:?Set POSTGRES_PASSWORD in .env}"
if [[ "$POSTGRES_PASSWORD" == "replace-with-a-local-development-secret" ]]; then
  printf 'Replace POSTGRES_PASSWORD in .env before running bootstrap.\n' >&2
  exit 1
fi

pnpm install --frozen-lockfile
docker compose up -d postgres

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

export DATABASE_URL="${DATABASE_URL:-postgres://smtp_dev:${POSTGRES_PASSWORD}@localhost:${POSTGRES_HOST_PORT:-5434}/smtp_platform?sslmode=disable}"
make migrate-up
cargo build --workspace
(cd smtp-api && go build ./...)
pnpm build:dashboard

printf '\nDevelopment bootstrap complete. Run ./scripts/run-dev.sh for normal starts.\n'
