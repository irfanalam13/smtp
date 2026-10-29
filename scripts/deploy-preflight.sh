#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ "${1:-}" != "--production-env" || $# -ne 1 ]]; then
  printf 'Usage: %s --production-env\n' "$0" >&2
  printf 'This validates a deployment environment; it does not deploy to a provider.\n' >&2
  exit 2
fi

for command in cargo go pnpm docker; do
  command -v "$command" >/dev/null 2>&1 || {
    printf 'Missing required command: %s\n' "$command" >&2
    exit 1
  }
done

[[ -n "${DATABASE_URL:-}" ]] || { printf 'DATABASE_URL is required.\n' >&2; exit 1; }
[[ "${CONTROL_PLANE_DEV_MODE:-false}" != "true" ]] || {
  printf 'CONTROL_PLANE_DEV_MODE must not be true for deployment.\n' >&2
  exit 1
}
[[ "${SMTP_DELIVERY_ENABLED:-false}" != "true" || -n "${SMTP_DELIVERY_ALLOWLIST:-}" ]] || {
  printf 'External SMTP delivery requires an explicit SMTP_DELIVERY_ALLOWLIST.\n' >&2
  exit 1
}

make check
cargo build --release --workspace
(cd smtp-api && go build -trimpath -ldflags="-s -w" ./...)
pnpm build:dashboard

printf '\nDeployment preflight passed. Publish the generated artifacts using your deployment platform.\n'
