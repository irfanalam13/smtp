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
if [[ "${SMTP_DELIVERY_ENABLED:-false}" == "true" ]]; then
  if [[ -n "${SMTP_RELAY_HOST:-}" || -n "${SMTP_RELAY_PORT:-}" ||
    -n "${SMTP_RELAY_USERNAME:-}" || -n "${SMTP_RELAY_PASSWORD:-}" ||
    -n "${SMTP_RELAY_FROM:-}" ]]; then
    [[ -n "${SMTP_RELAY_PORT:-}" && -n "${SMTP_RELAY_USERNAME:-}" &&
      -n "${SMTP_RELAY_PASSWORD:-}" && -n "${SMTP_RELAY_FROM:-}" ]] || {
      printf 'SMTP relay delivery requires host, port, username, password, and From settings.\n' >&2
      exit 1
    }
  elif [[ -z "${SMTP_DELIVERY_ALLOWLIST:-}" ]]; then
    printf 'Direct SMTP delivery requires an explicit SMTP_DELIVERY_ALLOWLIST.\n' >&2
    exit 1
  fi
fi

make check
cargo build --release --workspace
(cd smtp-api && go build -trimpath -ldflags="-s -w" ./...)
pnpm build:dashboard

printf '\nDeployment preflight passed. Publish the generated artifacts using your deployment platform.\n'
