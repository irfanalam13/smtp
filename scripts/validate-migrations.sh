#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?Set DATABASE_URL to an isolated PostgreSQL database}"
command -v docker >/dev/null 2>&1 || {
  printf '%s\n' 'Docker is required to validate migrations.' >&2
  exit 1
}

migration_dir=$(mktemp -d)
trap 'rm -rf "$migration_dir"' EXIT
"$PWD/scripts/stage-migrations.sh" "$migration_dir"

docker run --rm --network host \
  -v "$migration_dir:/migrations:ro" \
  migrate/migrate:v4.18.3 \
  -path=/migrations -database "$DATABASE_URL" up

printf '%s\n' 'Migrations are applied and the database is at the latest version.'
