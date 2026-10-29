# Local infrastructure

## PostgreSQL

PostgreSQL is the only local shared dependency currently required by the
repository. It supports the existing queue, control-plane migrations, and
opt-in inbound persistence prototypes.

| Property        | Value                                     |
| --------------- | ----------------------------------------- |
| Compose service | `postgres`                                |
| Image           | `postgres:17`                             |
| Host address    | `127.0.0.1:${POSTGRES_HOST_PORT:-5432}`   |
| Database        | `smtp_platform`                           |
| User            | `smtp_dev`                                |
| Password        | `POSTGRES_PASSWORD` from untracked `.env` |
| Volume          | `postgres_data`                           |
| Readiness       | `pg_isready -U smtp_dev -d smtp_platform` |

Start and inspect it with:

```bash
make infra-up
make infra-ps
```

Stop it without deleting data:

```bash
make infra-down
```

Apply the existing migrations with `make migrate-up` after setting
`DATABASE_URL`. Roll back one migration only against an intentionally
disposable local database with `make migrate-down`.

Validate an already running isolated database with `make migrate-validate`.
Create a local logical backup with `make backup-postgres`; see the
[backup and recovery runbook](../storage/backup-recovery.md).

Delete local data explicitly only when required:

```bash
docker compose down --volumes
```

Do not run the volume-removal command against production or a shared database.

## Why other services are absent

Redis, an object-storage emulator, and a dedicated event broker are not
included because Phase 01 selected PostgreSQL queue/outbox infrastructure
initially and the current code does not require the other services. Adding
them now would create contributor and operational cost without an executable
consumer. They can be added in the phase that introduces their actual
interfaces and tests.

## Network and security boundaries

The database port is bound to localhost. Compose does not publish an SMTP,
API, or dashboard service; those processes are run directly by
`scripts/run-dev.sh`. Do not add broad host mounts, Docker socket mounts, or
privileged containers to local services.
