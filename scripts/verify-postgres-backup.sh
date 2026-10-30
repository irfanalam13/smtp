#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'Usage: %s <backup.dump>\n' "$0" >&2
  exit 2
fi

backup="$1"
manifest="${backup}.sha256"

if [[ ! -f "$backup" ]]; then
  printf 'Backup file does not exist: %s\n' "$backup" >&2
  exit 1
fi
if [[ ! -f "$manifest" ]]; then
  printf 'SHA-256 manifest does not exist: %s\n' "$manifest" >&2
  exit 1
fi
command -v sha256sum >/dev/null 2>&1 || {
  printf '%s\n' 'sha256sum is required to verify this backup.' >&2
  exit 1
}

(
  cd "$(dirname "$backup")"
  sha256sum --check "$(basename "$manifest")"
)
