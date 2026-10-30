# Development setup

## Supported tools

- Rust stable with `rust-toolchain.toml`
- Go 1.26.10
- Node.js 20 and pnpm 12.15.0
- Docker Compose v2 or newer

The current target is local development on an Ubuntu laptop; cloud hosting is
not required. The dashboard, API, SMTP listener, and PostgreSQL port should
remain bound to loopback. The development runner rejects a non-loopback API
address. External SMTP delivery is disabled by default, and the queue worker
uses the local deterministic processor.

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

Open the dashboard at `http://127.0.0.1:3000`; the API is at
`http://127.0.0.1:8080`, and local SMTP listens on `127.0.0.1:2525`. These
addresses are for this laptop only, not LAN or public access.

`NEXT_PUBLIC_BRAND_NAME` sets the deployment-level dashboard label and page
metadata; the default is `SMTP Platform`. It is a public build-time display
value, not a secret or tenant-level branding feature. Changing it for a
production deployment requires rebuilding the dashboard.

The dashboard sends API requests through a same-origin Next.js rewrite.
`API_URL` sets the backend origin used by the dashboard server and defaults to
`http://127.0.0.1:8080`; set it to the private API origin when running the
dashboard and API on separate hosts. Do not expose credentials in a
`NEXT_PUBLIC_*` variable.

The generated `.env` keeps `CONTROL_PLANE_DEV_MODE=false` by default. To use
the local dashboard and development principal, set it to `true` before
starting the runner. This enables the `X-Dev-Subject` adapter only for local
development for non-mailbox control-plane routes; it cannot authorize
PostgreSQL mailbox reads. Never copy this setting into a deployment environment.
For durable mailbox access, paste a user-bound API key in the dashboard. Its
send form requires `delivery:write`; create or bootstrap keys only for an
active tenant member and grant the narrowest needed scopes.

Dashboard domain registration and TXT verification use PostgreSQL for durable
accounts; the local development principal uses process-local state. Usage
counters remain process-local. Message submissions are persisted to the queue,
but the development worker simulates results and never performs external SMTP
delivery. Durable submissions require a verified sender domain.

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

The control-plane API requires `DATABASE_URL` and a successful PostgreSQL
connection at startup. Existing domain and usage operations still use
development in-memory state. Mailbox list/message reads use the durable
PostgreSQL repository and require a user-bound API key. After applying
migrations, provision an initial key for an existing tenant member using the
[API bootstrap instructions](../api/README.md).

Other development-only API routes can still be exercised with the
development identity adapter:

```sh
export CONTROL_PLANE_DEV_MODE=true
export APP_ENVIRONMENT=development
go run ./smtp-api/cmd/api
curl -H 'X-Dev-Subject: alice' http://localhost:8080/v1/me
```

This adapter is not authentication for production and cannot authorize
PostgreSQL mailbox reads. Domains begin in `pending_verification`. Migrations,
including the user binding required by durable API keys, are applied by the
normal `make migrate-up` command. Remote production PostgreSQL connections
must use TLS; the local Compose configuration is loopback-only and may use
`sslmode=disable`.
