# Requirements-to-evidence matrix

Validation date: 2026-10-29. Commands were run from the repository root unless
the command includes a working-directory prefix.

| Requirement | Component | Validation method | Actual result | Status | Evidence/limitation |
|---|---|---|---|---|---|
| Rust formatting | Rust workspace | `cargo fmt --all -- --check` | Passed | PASS | Command output |
| Rust linting | Rust workspace | `cargo clippy --workspace --all-targets -- -D warnings` | Passed | PASS | Command output |
| Rust compile/build | Rust workspace | `cargo check --workspace --all-targets`; `cargo build --workspace --all-targets` | Passed | PASS | Command output |
| Rust unit/integration tests | Rust workspace | `cargo test --workspace --all-targets` | Passed; four PostgreSQL tests ignored without test DB in baseline | PASS with limitation | Isolated DB run below |
| Rust documentation tests | Rust workspace | `cargo test --workspace --doc` | Passed | PASS | Command output |
| Go formatting/lint/build | `smtp-api` | `gofmt -l .`, `go vet ./...`, `go build ./...` | Passed | PASS | Command output |
| Go functional tests | `smtp-api` | `go test ./...` | Passed | PASS | API package tests |
| Go race tests | `smtp-api` | `go test -race ./...` | Passed | PASS | Command output |
| OpenAPI syntax and required routes | API contract | Python YAML parse and route assertions | Passed | PASS | `docs/api/openapi.yaml` |
| Dashboard definition syntax | Grafana definition | Python JSON parse | Passed | PASS | `observability/grafana/api-overview.json` |
| Compose syntax | Local infrastructure | `docker compose config --quiet` | Passed | PASS | Command output |
| Configuration URL shape | Local/CI configuration | `scripts/validate-foundation-config.sh` | Passed | PASS | Script output |
| API liveness/readiness | Go API | Live server plus `curl /health/live` and `/health/ready` | Both returned 200 | PASS | Live validation |
| API metrics | Go API | Live server plus `curl /metrics` | Metrics endpoint returned 200 and Prometheus text | PASS | Live validation |
| API auth boundary | Go API | 100 requests to `/v1/domains` without credentials | All returned 401; no 5xx | PASS | Performance run |
| Fresh migrations | PostgreSQL | Disposable PostgreSQL 17 plus `validate-migrations.sh` | All five migrations applied | PASS | Isolated port 55433 |
| Queue persistence | PostgreSQL queue | Four ignored queue integration tests | 4 passed | PASS | Isolated disposable DB |
| Backup readability | PostgreSQL | `backup-postgres.sh`, `pg_restore --list` | Passed; expected tables present | PASS | Disposable DB |
| Backup restoration | PostgreSQL | Restore into new database and schema query | Passed | PASS | Disposable DB |
| Authentication/authorization | Control plane | Go security and control-plane tests | Passed | PASS | Existing Phase 07 tests |
| Tenant isolation | Control plane | Existing cross-tenant tests | Passed | PASS | Existing Phase 07 tests |
| Secret scanning | Repository | Phase 05 Gitleaks workflow | Not run locally; tool/backend unavailable | BLOCKED | Must run in CI |
| Rust dependency advisories | Rust lockfile | `cargo audit` | 2 vulnerabilities and 1 unmaintained warning | FAIL | See risk register |
| Fixed XML parser advisories | Rust lockfile | `cargo audit` after `quick-xml` 0.41.0 upgrade | quick-xml advisories cleared | PASS | `Cargo.toml`, `Cargo.lock` |
| POP3 release scope | Protocol scope | Review crate, README, and release docs | Explicitly out of scope; no listener or mailbox service | PASS with limitation | `smtp-pop3/README.md` |
| Production OIDC | Control plane | No provider configured | Not implemented | NOT TESTED | Production prerequisite |
| Durable control-plane repositories | Go/PostgreSQL | Current store is in-memory | Not implemented | NOT TESTED | Phase 06/08 handoff |
| Distributed tracing | Cross-service | No collector or second deployed service | Not implemented | BLOCKED | Phase 09/10 deployment prerequisite |
| Full load/capacity validation | Platform | Controlled local baseline only | No production target or representative environment | BLOCKED | Phase 10 follow-up |
| CI remote settings/branch protection | GitHub repository | Remote settings not inspected | Not verified | NOT TESTED | Requires repository access |

## Interpretation

The foundation has strong local build, unit, API, migration, queue, backup,
restore, and race-test evidence. It is not production-ready because the
dependency audit fails and several production controls remain unimplemented or
unverifiable.
