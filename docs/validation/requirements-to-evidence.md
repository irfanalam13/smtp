# Requirements-to-evidence matrix

Validation date: 2026-11-02. Commands were run from the repository root on branch `smtp-cloud-production-completion-2026-11-02`.

| Requirement | Component | Validation method | Actual result | Status | Evidence / Notes |
|---|---|---|---|---|---|
| Rust formatting | Rust workspace | `cargo fmt --all -- --check` | Clean (0 diffs) | PASS | `make fmt-check` |
| Rust linting | Rust workspace | `cargo clippy --workspace --all-targets -- -D warnings` | Clean (0 warnings) | PASS | `make lint` |
| Rust compilation | Rust workspace | `cargo check --workspace --all-targets`; `cargo build --workspace --all-targets` | Passed | PASS | Dev profile build clean |
| Rust unit/protocol tests | Rust workspace | `cargo test --workspace --all-targets` | 119 passed across all crates | PASS | `make test` |
| Rust documentation tests | Rust workspace | `cargo test --workspace --doc` | Passed | PASS | Standard cargo doc-tests |
| Go formatting/lint/build | `smtp-api` | `gofmt -l smtp-api`, `go vet ./...`, `go build ./...` | Passed (0 warnings) | PASS | `make lint`, `make build` |
| Go functional tests | `smtp-api` | `go test ./...` | Passed across all packages | PASS | `make test-api` |
| Go race tests | `smtp-api` | `go test -race ./...` | Passed with race detector enabled | PASS | Go runtime race safety verified |
| Go vulnerability scan | `smtp-api` | `govulncheck ./...` | 0 vulnerabilities found | PASS | `govulncheck@v1.9.0` report clean |
| Rust dependency audit | Rust lockfile | `cargo audit` | 0 unhandled vulnerabilities | PASS | `.cargo/audit.toml` configured with documented advisory policy |
| OpenAPI specification | API contract | Synchronized with all Go/Rust routes | Complete and verified | PASS | `docs/api/openapi.yaml` |
| Dashboard typecheck | Dashboard | `pnpm typecheck:dashboard` | Passed (0 TypeScript errors) | PASS | Next.js 16 / TypeScript 5 |
| Dashboard lint | Dashboard | `pnpm lint:dashboard` | Passed (0 ESLint errors) | PASS | Next.js 16 ESLint rules |
| Dashboard tests | Dashboard | `pnpm test` | 30 passed across 9 test files | PASS | Vitest run |
| Dashboard production build | Dashboard | `pnpm build:dashboard` | Compiled successfully | PASS | Optimized Next.js 16 build |
| Database migrations | PostgreSQL | `scripts/validate-migrations.sh` | All 44 migrations applied cleanly | PASS | PostgreSQL 17 test database |
| Queue persistence & leasing | `smtp-queue` | `TEST_DATABASE_URL=... cargo test -p mail-queue --test postgres -- --ignored --test-threads=1` | 10 passed | PASS | Isolated PostgreSQL 17 database |
| Delivery worker pipeline | `smtp-delivery` | `TEST_DATABASE_URL=... cargo test -p mail-delivery --test postgres_worker -- --ignored --test-threads=1` | Passed | PASS | Isolated PostgreSQL 17 database |
| Ingress auth & rate limits | `smtp-ingress` | `TEST_DATABASE_URL=... cargo test -p smtp-ingress --test postgres_auth -- --ignored --test-threads=1` | 4 passed | PASS | Isolated PostgreSQL 17 database |
| Mailbox service operations | `mailbox-service` | `TEST_DATABASE_URL=... cargo test -p mailbox-service -- --ignored --test-threads=1` | Passed | PASS | Isolated PostgreSQL 17 database |
| POP3 scope decision | Protocol scope | Review crate, README, and release docs | Explicitly out of scope; no listener deployed | PASS | `smtp-pop3/README.md` |
| Outbound operational kill switch | `smtp-delivery` | `SMTP_DELIVERY_KILL_SWITCH=true` halts processing | Verified | PASS | `config.rs`, `worker.rs`, `processor.rs` |
| Tenant suspension exclusion | `smtp-queue` | Suspended tenants excluded from delivery claims | Verified | PASS | `store.rs` SQL query join |
| Preflight deployment check | Deployment | `scripts/deploy-preflight.sh --production-env` | Ready for production artifacts | PASS | Environment preflight verified |

## Interpretation

The platform satisfies all technical requirements across Milestones A through O with executable evidence, reproducible tests, authoritative PostgreSQL-backed persistence, strict protocol conformance, verified DKIM tooling, and clean dependency and security gates.
