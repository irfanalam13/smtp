# Testing guide

## Standard validation

Run the repository validation before merging:

```sh
make check
```

This checks Rust formatting, Clippy, Rust tests, Go tests, Go vet, dashboard
linting, dashboard type checking, and the dashboard production build.

## Focused checks

```sh
make test-api
make test-delivery
cargo test -p smtp-ingress
```

Use focused checks during iteration and `make check` before handoff.

## PostgreSQL integration tests

Queue integration tests are ignored unless an explicit disposable database is
provided:

```sh
export TEST_DATABASE_URL='postgres://smtp_dev:password@127.0.0.1:5432/smtp_platform_test?sslmode=disable'
make test-queue
```

The tests apply or expect the repository migrations and may leave test rows.
Never use a personal, shared, staging, or production database.

## What is not covered yet

- Production OIDC flows.
- Go control-plane PostgreSQL repositories.
- Multi-instance quota correctness.
- A real outbound send endpoint.
- DKIM signing and verification.
- Full inbound PostgreSQL end-to-end coverage is opt-in via
  `TEST_DATABASE_URL`; the ignored integration fixture now exercises
  same-tenant primary/alias fan-out, cross-tenant atomic rejection, quota
  rollback, outbox creation, and shared per-domain rate enforcement.
- Load, chaos, backup-restore, and public-network SMTP tests.

These are release-blocking gaps, not reasons to weaken existing safety checks.
