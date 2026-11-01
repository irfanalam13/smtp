#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

worker_enabled=false
dashboard_enabled=true

usage() {
  cat <<'EOF'
Usage: ./scripts/run-dev.sh [OPTIONS]

Options:
  --worker         Start the outbound delivery worker
  --no-worker      Do not start the outbound delivery worker (default)
  --dashboard      Start the Next.js dashboard (default)
  --no-dashboard   Do not start the dashboard (useful if already running separately)
  -h, --help       Show this help message
EOF
  exit "${1:-0}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --worker)
      worker_enabled=true
      shift
      ;;
    --no-worker)
      worker_enabled=false
      shift
      ;;
    --dashboard)
      dashboard_enabled=true
      shift
      ;;
    --no-dashboard)
      dashboard_enabled=false
      shift
      ;;
    -h|--help)
      usage 0
      ;;
    *)
      printf 'Unknown option: %s\n' "$1" >&2
      usage 2
      ;;
  esac
done

for command in docker cargo go pnpm python3; do
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

# Detect duplicate or conflicting environment variables without exposing secrets
validate_env_duplicates() {
  python3 - <<'PY'
import sys, collections

seen = collections.defaultdict(list)
with open(".env") as f:
    for idx, line in enumerate(f, 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" in line:
            k, v = line.split("=", 1)
            k = k.strip()
            v = v.strip()
            seen[k].append((idx, v))

for k, occurrences in seen.items():
    if len(occurrences) > 1:
        vals = [v for _, v in occurrences]
        line_nums = ", ".join(str(idx) for idx, _ in occurrences)
        if all(v == vals[0] for v in vals):
            print(f"Notice: Duplicate variable '{k}' detected in .env on lines {line_nums} (identical values).", file=sys.stderr)
        else:
            print(f"Warning: Conflicting variable '{k}' detected in .env on lines {line_nums} (different values; last line takes precedence).", file=sys.stderr)
PY
}

validate_env_duplicates

set -a
# shellcheck disable=SC1091
source .env
set +a

: "${POSTGRES_PASSWORD:?Set POSTGRES_PASSWORD in .env}"
: "${POSTGRES_HOST_PORT:=5434}"
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
  printf 'For relay delivery, configure SMTP_RELAY_* settings instead.\n' >&2
  exit 1
fi

local_mailpit=false
if [[ "$APP_ENVIRONMENT" == "development" &&
  "${SMTP_RELAY_HOST:-}" == "127.0.0.1" &&
  "${SMTP_RELAY_PORT:-}" == "1025" &&
  -z "${SMTP_RELAY_USERNAME:-}" &&
  -z "${SMTP_RELAY_PASSWORD:-}" &&
  -n "${SMTP_RELAY_FROM:-}" ]]; then
  local_mailpit=true
fi

if [[ "$worker_enabled" == true && "$local_mailpit" == true ]] &&
  ! command -v mailpit >/dev/null 2>&1; then
  printf 'Missing required command: mailpit\n' >&2
  printf 'Install Mailpit from https://mailpit.axllent.org/docs/install/\n' >&2
  exit 1
fi

if [[ "$worker_enabled" == true && "$local_mailpit" != true &&
  ( -n "${SMTP_RELAY_HOST:-}" || -n "${SMTP_RELAY_PORT:-}" ||
    -n "${SMTP_RELAY_USERNAME:-}" || -n "${SMTP_RELAY_PASSWORD:-}" ||
    -n "${SMTP_RELAY_FROM:-}" ) ]]; then
  if [[ -z "${SMTP_RELAY_PORT:-}" || -z "${SMTP_RELAY_USERNAME:-}" ||
    -z "${SMTP_RELAY_PASSWORD:-}" || -z "${SMTP_RELAY_FROM:-}" ]]; then
    printf 'SMTP relay delivery requires host, port, username, password, and From settings in .env.\n' >&2
    exit 1
  fi
fi

# Port conflict detection before launching services
check_port() {
  local port="$1"
  python3 -c "
import socket, sys
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
try:
    s.bind(('127.0.0.1', int(sys.argv[1])))
    s.close()
    sys.exit(0)
except OSError:
    sys.exit(1)
" "$port"
}

get_port_owner() {
  local port="$1"
  python3 -c "
import subprocess, re, sys
port = sys.argv[1]
try:
    out = subprocess.check_output(['ss', '-tlpn', f'sport = :{port}'], text=True)
    match = re.search(r'pid=(\d+)', out)
    if match:
        pid = match.group(1)
        try:
            cmd = subprocess.check_output(['ps', '-p', pid, '-o', 'args='], text=True).strip()
            print(f'{cmd} (PID {pid})')
            sys.exit(0)
        except Exception:
            print(f'PID {pid}')
            sys.exit(0)
except Exception:
    pass
print('unknown process')
" "$port"
}

has_port_conflict=false

# Check API port (8080)
api_port="${API_ADDR##*:}"
if ! check_port "$api_port"; then
  owner="$(get_port_owner "$api_port")"
  printf 'Error: Port %s required by API is already in use by %s.\n' "$api_port" "$owner" >&2
  printf '  Stop the existing process with: kill <PID>\n' >&2
  has_port_conflict=true
fi

# Check Mailbox service port (8081)
mailbox_port="${MAILBOX_SERVICE_ADDR##*:}"
if ! check_port "$mailbox_port"; then
  owner="$(get_port_owner "$mailbox_port")"
  printf 'Error: Port %s required by Mailbox service is already in use by %s.\n' "$mailbox_port" "$owner" >&2
  printf '  Stop the existing process with: kill <PID>\n' >&2
  has_port_conflict=true
fi

# Check SMTP ingress port (2525)
if ! check_port "2525"; then
  owner="$(get_port_owner "2525")"
  printf 'Error: Port 2525 required by SMTP ingress is already in use by %s.\n' "$owner" >&2
  printf '  Stop the existing process with: kill <PID>\n' >&2
  has_port_conflict=true
fi

# Check Dashboard port (3000) if dashboard is enabled
if [[ "$dashboard_enabled" == true ]]; then
  if ! check_port "3000"; then
    owner="$(get_port_owner "3000")"
    printf 'Error: Port 3000 required by Dashboard is already in use by %s.\n' "$owner" >&2
    printf '  Safe recovery options:\n' >&2
    printf '    1. If the existing dashboard is already running, run without dashboard:\n' >&2
    printf '       ./scripts/run-dev.sh --no-dashboard\n' >&2
    printf '    2. Or stop the process using port 3000:\n' >&2
    printf '       kill <PID>\n' >&2
    has_port_conflict=true
  fi
fi

# Check Mailpit ports when the local relay is enabled
if [[ "$worker_enabled" == true && "$local_mailpit" == true ]]; then
  for mailpit_port in 1025 8025; do
    if ! check_port "$mailpit_port"; then
      owner="$(get_port_owner "$mailpit_port")"
      printf 'Error: Port %s required by Mailpit is already in use by %s.\n' "$mailpit_port" "$owner" >&2
      printf '  Stop the existing process with: kill <PID>\n' >&2
      has_port_conflict=true
    fi
  done
fi

if [[ "$has_port_conflict" == true ]]; then
  printf '\nAborting startup due to occupied required ports. Unrelated processes were not killed.\n' >&2
  exit 1
fi

children=()
kill_process_tree() {
  local pid="$1"
  local sig="${2:-TERM}"
  local child_pid
  for child_pid in $(pgrep -P "$pid" 2>/dev/null); do
    kill_process_tree "$child_pid" "$sig"
  done
  kill -"$sig" "$pid" 2>/dev/null || true
}

cleanup() {
  trap - EXIT INT TERM
  printf '\nStopping development services...\n'
  local child
  for child in "${children[@]}"; do
    if [[ -n "$child" ]]; then
      kill -TERM -"$child" 2>/dev/null || true
      kill_process_tree "$child" TERM 2>/dev/null || true
    fi
  done

  local wait_count=0
  local any_alive=true
  while [[ "$wait_count" -lt 20 && "$any_alive" == true ]]; do
    any_alive=false
    for child in "${children[@]}"; do
      if kill -0 "$child" 2>/dev/null; then
        any_alive=true
        break
      fi
    done
    if [[ "$any_alive" == true ]]; then
      sleep 0.1
      ((wait_count++)) || true
    fi
  done

  for child in "${children[@]}"; do
    if [[ -n "$child" ]]; then
      kill -9 -"$child" 2>/dev/null || true
      kill_process_tree "$child" KILL 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true
  printf 'All application processes stopped. PostgreSQL remains running.\n'
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
printf 'PostgreSQL is healthy.\n'

start_process() {
  local name="$1"
  shift
  printf 'Starting %s...\n' "$name"
  setsid "$@" &
  local pid=$!
  children+=("$pid")
  printf '%s process started with PID %s.\n' "$name" "$pid"
  STARTED_PROCESS_PID="$pid"
}

if [[ "$worker_enabled" == true && "$local_mailpit" == true ]]; then
  start_process "Mailpit" mailpit --smtp 127.0.0.1:1025 --listen 127.0.0.1:8025
  printf 'Waiting for Mailpit...\n'
  for attempt in {1..20}; do
    if python3 -c "
import socket, sys
for port in (1025, 8025):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1.0)
        try:
            s.connect(('127.0.0.1', port))
        except OSError:
            sys.exit(1)
sys.exit(0)
"; then
      break
    fi
    if [[ "$attempt" -eq 20 ]]; then
      printf 'Mailpit did not become ready on ports 1025 and 8025.\n' >&2
      exit 1
    fi
    sleep 0.5
  done
  printf 'Mailpit is healthy (SMTP 127.0.0.1:1025, UI http://127.0.0.1:8025).\n'
fi

start_process "API" go run ./smtp-api/cmd/api
start_process "mailbox service" cargo run -p mailbox-service
mailbox_process_pid="$STARTED_PROCESS_PID"
printf 'Waiting for mailbox service on %s...\n' "$MAILBOX_SERVICE_ADDR"
mailbox_host="${MAILBOX_SERVICE_ADDR%:*}"
mailbox_port="${MAILBOX_SERVICE_ADDR##*:}"
for attempt in {1..30}; do
  if python3 -c "
import socket, sys
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.settimeout(1.0)
    try:
        s.connect((sys.argv[1], int(sys.argv[2])))
    except OSError:
        sys.exit(1)
" "$mailbox_host" "$mailbox_port"; then
    break
  fi
  if ! kill -0 "$mailbox_process_pid" 2>/dev/null; then
    printf 'Mailbox service exited before becoming ready. Check its startup error above.\n' >&2
    exit 1
  fi
  if [[ "$attempt" -eq 30 ]]; then
    printf 'Mailbox service did not become ready on %s.\n' "$MAILBOX_SERVICE_ADDR" >&2
    exit 1
  fi
  sleep 0.5
done
printf 'Mailbox service is healthy on %s.\n' "$MAILBOX_SERVICE_ADDR"
if [[ "$dashboard_enabled" == true ]]; then
  start_process "dashboard" pnpm dev:dashboard
fi
start_process "SMTP ingress" cargo run -p smtp-ingress
if [[ "$worker_enabled" == true ]]; then
  start_process "outbound delivery worker" cargo run -p mail-delivery --bin mail-delivery
fi

cat <<EOF

Development services are running:
  API:             http://${API_ADDR}
EOF
if [[ "$dashboard_enabled" == true ]]; then
  printf '  Dashboard:       http://localhost:3000\n'
else
  printf '  Dashboard:       (external or disabled; port 3000)\n'
fi
cat <<EOF
  Mailbox service: http://${MAILBOX_SERVICE_ADDR}
  SMTP ingress:    127.0.0.1:2525
EOF
if [[ "$worker_enabled" == true ]]; then
  if [[ "$local_mailpit" == true ]]; then
    printf '  Delivery worker: active (Mailpit relay at 127.0.0.1:1025)\n'
    printf '  Mailpit UI:      http://127.0.0.1:8025\n'
  else
    printf '  Delivery worker: active\n'
  fi
fi
printf '  Database:        localhost:%s\n\n' "$POSTGRES_HOST_PORT"
cat <<'EOF'
Use Ctrl-C to stop application processes. PostgreSQL remains running for the
next start; use `make dev-down` when you want to stop it.
EOF

status=0
wait -n "${children[@]}" || status=$?
printf 'A development process exited with status %d; stopping the remaining processes.\n' "$status" >&2
exit "$status"
