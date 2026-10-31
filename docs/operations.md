# Operations guide

## Local service map

| Service | Address | Start command |
| --- | --- | --- |
| PostgreSQL | `127.0.0.1:5432` | Docker Compose |
| Go API | `http://127.0.0.1:8080` | `go run ./smtp-api/cmd/api` |
| Dashboard | `http://127.0.0.1:3000` | `pnpm dev:dashboard` |
| SMTP ingress | `127.0.0.1:2525` | `cargo run -p smtp-ingress` |
| Outbound delivery worker | optional; external delivery disabled by default | `cargo run -p mail-delivery` |

Use `./scripts/run-dev.sh` instead of starting these processes manually.

## First startup

Run `./scripts/bootstrap-dev.sh`. It creates a local `.env` when needed,
installs locked dependencies, starts PostgreSQL, applies migrations, and builds
the workspaces.

## Normal startup and shutdown

Run `./scripts/run-dev.sh`. Press `Ctrl-C` to stop application processes.
PostgreSQL intentionally remains running so the next start is fast. Stop it
with:

```sh
make dev-down
```

The development runner does not start outbound delivery by default. To run
the real worker, configure `SMTP_DELIVERY_ENABLED=true` and an explicit
`SMTP_DELIVERY_ALLOWLIST` in `.env`, then start with
`./scripts/run-dev.sh --worker`. The worker sends real external SMTP.

## Health and diagnostics

Check the API liveness endpoint:

```sh
curl http://127.0.0.1:8080/health/live
```

Check PostgreSQL readiness:

```sh
docker compose exec postgres pg_isready -U smtp_dev -d smtp_platform
```

With development mode enabled, inspect the control-plane identity:

```sh
curl -H 'X-Dev-Subject: alice' http://127.0.0.1:8080/v1/me
```

Development identity mode must never be enabled in production.

## Migration recovery

Apply pending migrations with:

```sh
make migrate-up
```

The migration command requires `DATABASE_URL`. Do not run
`make migrate-down` against shared or production data without an approved
rollback plan.

## Failure handling

If a service exits, inspect its terminal output from the runner and verify
PostgreSQL readiness. A stale local process or occupied port can be removed
only after identifying the owning process. Do not use broad process-killing
commands.

If migrations fail, preserve the error output, verify `DATABASE_URL`, and
inspect migration state before retrying. Do not delete the PostgreSQL volume
unless all local data can be discarded.

## Production boundary

`deploy-preflight.sh` validates configuration and builds release artifacts; it
does not deploy. A production runbook still needs a selected hosting target,
secret management, database backup/restore, TLS, monitoring, alerting,
rollback, and incident-response procedures.
