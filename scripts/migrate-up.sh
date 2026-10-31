#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

exported_database_url="${DATABASE_URL:-}"
if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi
if [[ -n "$exported_database_url" ]]; then
  DATABASE_URL="$exported_database_url"
fi
: "${DATABASE_URL:?Set DATABASE_URL in .env or the environment}"

python3 - "$DATABASE_URL" <<'PY'
import sys
from urllib.parse import urlsplit

try:
    target = urlsplit(sys.argv[1])
    valid = (
        target.scheme in {"postgres", "postgresql"}
        and target.hostname in {"localhost", "127.0.0.1", "::1"}
        and target.username == "smtp_dev"
        and target.path.lstrip("/") == "smtp_platform"
    )
except ValueError:
    valid = False

if not valid:
    print(
        "Refusing migration: DATABASE_URL must target the local "
        "smtp_platform database as smtp_dev.",
        file=sys.stderr,
    )
    raise SystemExit(1)
PY

command -v psql >/dev/null 2>&1 || {
  printf '%s\n' 'psql is required to verify the application database before migration.' >&2
  exit 1
}
command -v docker >/dev/null 2>&1 || {
  printf '%s\n' 'Docker is required to apply migrations.' >&2
  exit 1
}

if ! database_identity="$(
  psql --no-psqlrc --no-align --tuples-only --set=ON_ERROR_STOP=1 \
    "$DATABASE_URL" \
    --command="SELECT current_database() || '|' || current_user" 2>/dev/null
)"; then
  printf '%s\n' 'Could not connect to the configured application database; migrations were not run.' >&2
  exit 1
fi
if [[ "$database_identity" != "smtp_platform|smtp_dev" ]]; then
  printf '%s\n' 'Refusing migration: the connected database identity is not smtp_platform/smtp_dev.' >&2
  exit 1
fi
printf '%s\n' 'Verified connection to the local smtp_platform application database.'

migration_dir="$(mktemp -d)"
trap 'rm -rf "$migration_dir"' EXIT
./scripts/stage-migrations.sh "$migration_dir"
docker run --rm --network host \
  -v "$migration_dir:/migrations:ro" \
  migrate/migrate:v4.18.3 \
  -path=/migrations -database "$DATABASE_URL" up
