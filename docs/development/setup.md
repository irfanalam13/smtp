# Development setup

## Supported tools

- Rust stable with `rust-toolchain.toml`
- Go 1.26.8
- Node.js 20 and pnpm 12.15.0
- Docker Compose v2 or newer

Copy `.env.example` to `.env`, replace the local password, and start PostgreSQL:

```sh
docker compose up -d postgres
export DATABASE_URL='postgres://smtp_dev:local-password@localhost:5432/smtp_platform?sslmode=disable'
make migrate-up
```

For the first run on a new checkout, use the bootstrap script:

```sh
./scripts/bootstrap-dev.sh
```

Bootstrap creates `.env` with a generated local database password when needed,
installs locked frontend dependencies, starts PostgreSQL, applies migrations,
and builds all workspaces. It is safe to rerun after pulling schema or
dependency changes.

For normal development starts after bootstrap, use:

```sh
./scripts/run-dev.sh
```

The normal runner starts PostgreSQL and launches the Go API, Next.js
dashboard, SMTP ingress, and deterministic queue worker without rerunning
migrations or installing dependencies. It stops application processes on
`Ctrl-C` and leaves PostgreSQL running. Use `make dev-down` to stop it. Use
`./scripts/run-dev.sh --no-worker` to omit the queue worker.

The generated `.env` keeps `CONTROL_PLANE_DEV_MODE=false` by default. To use
the local dashboard and development principal, set it to `true` before
starting the runner. This enables the `X-Dev-Subject` adapter only for local
development; never copy this setting into a deployment environment.

Before deploying, run the production-safe build and configuration checks:

```sh
export DATABASE_URL='postgres://...'
export CONTROL_PLANE_DEV_MODE=false
./scripts/deploy-preflight.sh --production-env
```

This preflight does not deploy to a cloud provider because no provider or
production runtime target is configured in this repository. It rejects the
development identity adapter, requires a database URL, checks external
delivery allowlisting, runs the full checks, and builds release artifacts.

Run `make check` for the application checks. Use `pnpm dev:dashboard` and
`go run ./smtp-api/cmd/api` for local application development.

Run the SMTP prototype locally with:

```sh
cargo run -p smtp-ingress
```

It binds only to `127.0.0.1:2525` by default and is not a production mail
service. Its protocol tests run with `cargo test -p smtp-ingress`.

Apply the queue migration with `make migrate-up`. Run the durable queue worker
locally with `cargo run -p mail-queue`; it uses the deterministic test
processor and never performs remote SMTP delivery. Queue integration tests are
destructive and require an explicitly isolated database:

```sh
export TEST_DATABASE_URL='postgres://smtp_dev:local-password@localhost:5432/smtp_platform_test?sslmode=disable'
make test-queue
```

Inspect queue state with `psql`; do not point `TEST_DATABASE_URL` at a database
containing non-test data. The queue retains test rows until explicitly cleaned
up.

`.env` is local-only and must never contain production credentials.

Inbound reception is not enabled by the default binary. To enable it in an
application, construct `PostgresInboundStore` with an isolated
`DATABASE_URL` and pass it as `Config.inbound`. The database must contain a
verified domain and active mailbox record before SMTP recipients will be
accepted. The listener remains bound to loopback unless `bind_addr` is
explicitly changed by the operator.

Milestone 6 domain checks use the system resolver with a three-second timeout
and a fifteen-minute result cache. `CONTROL_PLANE_DEV_MODE=true` remains
development-only. The current quota is 1,000 message submissions per tenant
per one-hour window and a 10 MiB maximum message size; it is single-process
until the control-plane store is moved to PostgreSQL.

The Milestone 4 control-plane API can be exercised locally with the
development-only adapter:

```sh
export CONTROL_PLANE_DEV_MODE=true
go run ./smtp-api/cmd/api
curl -H 'X-Dev-Subject: alice' http://localhost:8080/v1/me
```

This adapter is not authentication for production. Domains begin in
`pending_verification`; the API-key secret is returned only from its create
response. Control-plane tables are applied by the normal `make migrate-up`
command.
