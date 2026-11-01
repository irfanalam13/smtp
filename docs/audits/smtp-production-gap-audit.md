# SMTP production gap audit

**Audit date:** 2026-11-01
**Branch:** `copilot/smtp-production-gap-remediation`
**Method:** source, migration, executable-entry-point and test inspection. A status is
not inferred from a README or a type declaration.

## Evidence map

| Area | Status | Runtime evidence | Tests / command | Gap and minimum safe next step |
|---|---|---|---|---|
| Password signup and token storage | PARTIAL | `smtp-api/internal/controlplane/password_auth_handlers.go`; `password_accounts_postgres.go`; migrations `000029`, `000034` | `go test ./internal/controlplane` | Tokens are hashed and single-use; responses are generic and account notifications now use an encrypted transactional outbox with leases/retries. Full disposable-Postgres outage/replay evidence remains required. |
| Password reset, resend and change-password | PARTIAL | `password_auth_handlers.go`; `password_accounts_postgres.go` | `go test ./internal/controlplane`; optional `TEST_DATABASE_URL` integration | Purpose, expiry and replay checks exist; notification retry/outbox, shared throttling and complete security-event revocation remain. |
| API abuse throttling | BROKEN for production scale | `Store.Allow` in `store.go`, called by auth middleware and auth handlers | unit tests only | Counters are process-local. Add a PostgreSQL atomic limiter with HMAC-pseudonymized bounded keys and fail closed when unavailable; retain local limits only for explicit dev/test. |
| OAuth/OIDC | PARTIAL | `auth_sessions_handlers.go`; `social_oauth.go`; `smtp-auth/oidc.go` | `go test ./...` | State comparison and OIDC PKCE/nonce paths exist; provider response/status bounds, server-side single-use flow records, exact redirect/provider validation and complete Google/GitHub protocol tests are missing. |
| JWT and browser sessions | PARTIAL | `jwt.go`; `auth_sessions_handlers.go`; `000028` | `go test ./internal/controlplane` | HS256 is pinned and opaque sessions are persisted; bearer key `kid` rotation, strict claim schema, refresh replay controls and security-event revocation need completion. |
| SMTP listener roles | PARTIAL | `smtp-session/src/runtime/config.rs`; `smtp-session/src/main.rs` | `cargo test -p smtp-ingress` | TLS-gated AUTH and loopback safety exist, but one environment-driven listener does not explicitly model port 25/587/465 roles. Add validated listener profiles and role-specific routing/capabilities. |
| SMTP core transaction | PARTIAL | `smtp-session/src/protocol/session.rs`, `command.rs`, `server.rs` | `cargo test -p smtp-ingress` | Bounds, transaction resets and durable queue hooks exist; PIPELINING, DSN, SMTPUTF8, CHUNKING and complete slow-client/failure matrix are not implemented/advertised. |
| Trusted `Received` trace | BROKEN | `smtp-session/src/inbound/mod.rs`; `docs/architecture/inbound-mail.md` | inbound Rust tests | Metadata is stored separately and accepted bytes are not traced. Add deterministic pre-commit trace generation, raw-wire/delivered-byte model and retry/DKIM fixtures. |
| Queue persistence and leasing | PARTIAL | `smtp-queue`; `smtp-storage/postgres/queue` migrations | `cargo test -p mail-queue` | Idempotent enqueue and recipient rows exist; lease-fencing, outage and uncertain post-DATA evidence need expansion. |
| Outbound SMTP | PARTIAL | `smtp-delivery`; `smtp-queue` | `cargo test --workspace` | Real direct-MX delivery, retry classification and DKIM exist; shared fairness/circuit breakers, complete DNS safety, MTA-STS/TLS-RPT and external fake-MX matrix remain. |
| DKIM lifecycle | PARTIAL | delivery signer and domain/control-plane handlers | workspace tests | Signing is present, but tenant-managed create/publish/verify/rotate/retire/revoke and secret-manager boundary are absent. |
| Inbound routing/scanning/authentication | PARTIAL | `smtp-session/src/inbound`; scanner and email-auth adapters | Rust workspace tests; disposable Postgres required for integration | Routing, fanout, scanners, rate limits and SPF/DKIM/DMARC adapters exist; durable event recovery, explicit DNS budget, attachment bounds and trusted Authentication-Results policy need completion. |
| DSN, bounces and suppression | PARTIAL | queue recipient state and submission APIs | targeted queue/API tests | Recipient outcomes exist; durable DSN generation, null reverse path, authenticated feedback, suppression race checks and webhooks need implementation. |
| Mailbox store | PARTIAL | `smtp-core/src/mailbox`; `mailbox-service`; mailbox migrations | `cargo test --workspace` | PostgreSQL metadata/content, flags, search and retention primitives exist; full lifecycle, object consistency and legal hold remain. |
| IMAP | PARTIAL | `smtp-imap/src/server`; runtime binds implicit TLS | `cargo test -p smtp-imap` | A bounded IMAP4rev1 profile is executable; independent-client matrix, all advertised-command conformance, drain/readiness metrics and account/device abuse tests remain. |
| POP3 | OUT-OF-SCOPE | `smtp-pop3` contains parser/session simulation only | `cargo test -p smtp-pop3` | No listener or mailbox adapter exists. Keep it out of release docs/config/UI until a real RFC 1939/RFC 8314 service is implemented. |
| Blob/object storage | MISSING | `smtp-storage/README.md`; inline mailbox schema | no end-to-end object-store test | Add provider-neutral adapter, local test adapter, atomic references, orphan repair, encryption metadata, retention and restore evidence before moving large content out of PostgreSQL. |
| Retention/deletion/legal hold | PARTIAL | `mailbox-service/src/retention.rs`; mailbox migrations | mailbox unit/integration tests | Current cleanup targets append-idempotency records, not message lifecycle. Add scheduled message/attachment retention, tombstones, holds, quota release and audited hard delete. |
| Deployment topology and DNS | DOCUMENTATION-ONLY | `compose.yaml`; `scripts/deploy-preflight.sh`; operations docs | `docker compose config --quiet`; script validation | Topology/preflight documentation exists; public DNS/rDNS/provider prerequisites cannot be verified in this repository. |
| Observability | PARTIAL | `smtp-session/src/runtime/metrics.rs`; API observability package; docs | service unit tests | Structured logs and metrics exist, but fleet aggregation, low-cardinality dashboards, traces, SLOs and complete alerts/runbooks need evidence. |
| Supply chain/release controls | PARTIAL | `.github/workflows/ci.yml`; security docs | CI workflow; local audit commands | CI runs core Rust/Go/dashboard checks; SBOM/provenance/image scanning and all advisory remediation need completion. |

## Required baseline commands

Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo check --workspace --all-targets`, `cargo test --workspace --all-targets`,
`cargo test --workspace --doc`, and `cargo build --workspace --all-targets`.

Go: from `smtp-api`, `test -z "$(gofmt -l .)"`, `go test ./...`, `go test -race ./...`,
`go vet ./...`, and `go build ./...`.

Dashboard: `pnpm install --frozen-lockfile`, `pnpm lint:dashboard`,
`pnpm typecheck:dashboard`, and `pnpm build:dashboard`.

Infrastructure: `docker compose config --quiet` and `./scripts/validate-migrations.sh`.
Database-dependent tests require an isolated disposable PostgreSQL instance; no production
database was used for this audit.

## Release decision

**NOT READY.** The repository has substantial working components, but production-scale
shared abuse controls, durable account notification delivery, listener role separation,
trace-header correctness, external protocol evidence, storage lifecycle and operational
acceptance gates are not all complete. External DNS, scanner, secret-manager, provider,
backup/restore and deployment validation remain prerequisites even after code gaps are fixed.
