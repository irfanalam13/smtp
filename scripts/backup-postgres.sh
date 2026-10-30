#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?Set DATABASE_URL}"
output="${1:-backups/postgres-$(date -u +%Y%m%dT%H%M%SZ).dump}"
mkdir -p "$(dirname "$output")"
umask 077

if command -v pg_dump >/dev/null 2>&1; then
  pg_dump --format=custom --file="$output" "$DATABASE_URL"
else
  command -v docker >/dev/null 2>&1 || {
    printf '%s\n' 'pg_dump or Docker is required to create a backup.' >&2
    exit 1
  }
  docker run --rm --network host \
    -e DATABASE_URL \
    -v "$PWD/$(dirname "$output"):/backup" \
    postgres:17 \
    pg_dump --format=custom --file="/backup/$(basename "$output")" "$DATABASE_URL"
fi

(
  cd "$(dirname "$output")"
  sha256sum "$(basename "$output")" > "$(basename "$output").sha256"
)

printf 'Created PostgreSQL backup and SHA-256 manifest: %s\n' "$output"
