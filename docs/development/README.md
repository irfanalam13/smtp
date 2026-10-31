# Development guide

## Supported workflow

The supported local workflow uses Git, the repository Rust toolchain, Go,
Node.js/pnpm, Docker Engine with Compose v2, and GNU Make. Linux is exercised
by the repository checks. macOS and Windows contributors should use Docker
Desktop or a Linux-compatible shell and verify platform-specific behavior
with `make doctor`.

The Rust toolchain is declared in [`../../rust-toolchain.toml`](../../rust-toolchain.toml).
The Go version is declared in [`../../smtp-api/go.mod`](../../smtp-api/go.mod).
The dashboard pins pnpm in [`../../package.json`](../../package.json).

## First checkout

```bash
cp .env.example .env
make doctor
make setup
```

`setup` installs the locked dashboard dependencies, starts PostgreSQL,
applies the existing migrations, and builds the existing workspaces. It does
not overwrite `.env` or install host packages.

## Environment

`.env.example` contains local-only placeholders. Copy it to `.env`; keep the
copy untracked. `POSTGRES_PASSWORD` is required by Compose. `DATABASE_URL`,
`API_ADDR`, and `CONTROL_PLANE_DEV_MODE` are used by the current local
prototype. Development identity mode is not production authentication.

The default PostgreSQL host port is 5434 to keep it separate from the
disposable test database on 5432. Change the PostgreSQL host port with
`POSTGRES_HOST_PORT` in `.env` and update the port in `DATABASE_URL` together.
If PostgreSQL port 5434 is occupied, for example use
`POSTGRES_HOST_PORT=55432` and
`DATABASE_URL=postgres://smtp_dev:<local-password>@localhost:55432/smtp_platform?sslmode=disable`.
Never solve a port or TLS problem by exposing the database publicly or
disabling certificate validation.

## Repository layout

| Path | Responsibility |
|---|---|
| `smtp-session/`, `smtp-delivery/`, `smtp-queue/`, `smtp-core/`, `smtp-imap/`, `smtp-pop3/` | Rust protocol, data-plane, and shared-core workspaces |
| `smtp-api/` | Go control-plane API |
| `smtp-dashboard/` | Next.js dashboard |
| `smtp-storage/` | PostgreSQL migrations |
| `smtp-ai/`, `smtp-analytics/`, `smtp-auth/`, `smtp-cli/`, `smtp-kubernetes/`, `smtp-monitoring/`, `smtp-parser/`, `smtp-sdk/`, `smtp-security/`, `smtp-tests/` | Planned or separately scoped boundaries; consult their contents before treating them as implemented workspaces |
| `scripts/` | Bootstrap, local-run, and preflight scripts |
| `docs/architecture/` | System architecture, repository structure, and implementation boundaries |
| `docs/standards/` | Phase 02 standards and RFC research |
| `docs/development/core-infrastructure.md` | Phase 04 shared-core contracts and adoption rules |
| `compose.yaml` | Local PostgreSQL dependency only |

See the [repository structure guide](../architecture/repository-structure.md)
for feature-level ownership and file organization. Do not add a component or
placeholder source files just to reserve a future boundary.

## Command reference

| Command | Behavior |
|---|---|
| `make help` | Show implemented developer commands |
| `make setup` | Validate prerequisites and bootstrap local dependencies |
| `make doctor` | Report tool versions and required repository files |
| `make infra-up` | Start PostgreSQL and wait for `pg_isready` |
| `make infra-down` | Stop containers while preserving named volumes |
| `make infra-ps` | Show container state and health |
| `make fmt` / `make fmt-check` | Format or check Rust, Go, and dashboard files |
| `make lint` | Run Clippy, Go vet, and dashboard lint |
| `make test` | Run Rust and Go tests |
| `make build` | Build Rust, Go, and dashboard artifacts |
| `make check` | Run formatting, lint, and test validation |
| `make clean` | Remove generated build artifacts only |

`make dev-run` starts the current local API, dashboard, SMTP ingress, and
optional queue worker. It is a prototype runner, not a production supervisor.

## Infrastructure lifecycle

The local Compose project contains only PostgreSQL because it is the only
currently required shared dependency. It uses the explicit `postgres:17`
image, localhost binding on configurable port 5434 by default, a named `postgres_data` volume, and a
native `pg_isready` health check.

Stopping with `make infra-down` preserves data. Recreating the container also
preserves the named volume. To intentionally delete local database data, use
the separately documented command:

```bash
docker compose down --volumes
```

This is destructive to the local PostgreSQL volume and is never performed by
`make infra-down`, `make clean`, or routine setup.

## Testing and troubleshooting

Run `make check` for the normal fast validation. Run `make build` for all
workspace and dashboard builds. Queue PostgreSQL integration tests are ignored
unless `TEST_DATABASE_URL` is set to an isolated disposable database; use the
CI migration workflow or a dedicated local test database.

If PostgreSQL is not ready:

```bash
make infra-ps
docker compose logs postgres
make infra-down
make infra-up
```

If dependencies are missing, run `make doctor` and install the specific
documented prerequisite. Do not commit generated directories or local
configuration to work around a missing tool.

## Remaining implementation

This phase provides the local workflow, not a production deployment
environment. Production manifests, cross-platform validation, and
production-like integration environments remain deferred until the runtime and
deployment target are selected. See the [phase implementation status](../phase-status.md)
under Phase 03 for reasons and application timing.
