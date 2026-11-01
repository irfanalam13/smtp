#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v python3 >/dev/null 2>&1; then
  printf 'python3 is required to run verify-mailpit.sh\n' >&2
  exit 1
fi

chmod +x scripts/verify-mailpit.py
exec python3 scripts/verify-mailpit.py "$@"
