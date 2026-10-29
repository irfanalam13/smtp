# DevOps and CI/CD

Phase 05 provides GitHub Actions validation and release packaging without
introducing production deployment infrastructure.

## Workflow inventory

| Workflow                                                      | Trigger                               | Purpose                                                                  |
| ------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------ |
| [CI](../../.github/workflows/ci.yml)                          | Pull requests, `main`, manual         | Rust, Go, dashboard, and PostgreSQL queue validation                     |
| [Security](../../.github/workflows/security.yml)              | Pull requests, `main`, weekly, manual | Dependency review, Rust advisories, secret scanning, workflow guardrails |
| [Container validation](../../.github/workflows/container.yml) | Pull requests, `main`, manual         | Compose syntax and tracked-secret checks                                 |
| [Release](../../.github/workflows/release.yml)                | `vX.Y.Z` tags, manual                 | Trusted source/binary packaging, checksums, metadata, GitHub Release     |

All workflows use read-only repository permissions by default. Only the release
job receives `contents: write`, and only on a trusted tag or explicitly
requested existing tag. No workflow deploys to production or accesses cloud
credentials.

## Local equivalence

Run the same core checks locally:

```bash
make fmt-check
make lint
make test
make build
cargo test --workspace --doc
docker compose -f compose.yaml config --quiet
```

The PostgreSQL integration job creates an isolated disposable service and
applies the repository migrations before running the ignored queue tests.
Local queue integration tests require `TEST_DATABASE_URL` and must not target a
normal development database.

## Caching and trust boundaries

Rust caching is keyed by the declared toolchain and lockfile through
`Swatinem/rust-cache`. pnpm and Go use the setup actions' lockfile-aware
caches. A cache miss must still produce a clean build.

Pull-request workflows have no publishing credentials. Release packaging is
separate from pull-request execution and checks out only a validated version
tag. Do not use `pull_request_target` for code execution.

## Failure diagnosis

Start with the failing job's first failed command, reproduce it with the local
equivalent above, and attach sanitized output to the pull request. Never attach
`.env`, credentials, message content, database dumps, or raw secret-scanner
matches. Dependabot updates must pass the same required checks as ordinary
changes.

## Remaining implementation

The workflows validate code and package releases but do not deploy production.
Image provenance, registry scanning, repository governance, deployment
promotion, rollback, and unresolved dependency remediation remain deferred
until a production release target is selected. See the [phase implementation
status](../phase-status.md) under Phase 05.
