# Backend platform audit (M00)

## Outcome

The repository is a useful local and integration foundation, not a production
email platform. It contains PostgreSQL-backed API and queue operations, SMTP
and outbound-delivery protocol code, and extensive mailbox/domain primitives.
Several important implementations are not connected to executable service
paths, and some development executables deliberately simulate success.

This document records the M00 initial audit and baseline. Follow-up
implementation changes are reflected in the current [phase status](phase-status.md).
It does not certify production readiness or substitute for PostgreSQL
integration, interoperability, load, or deployment testing.

## Service integration matrix

| Area                                               | Executable / entry point                                                                | Persistence and configuration                                                                                                                                                                                                                                                                                                                                                | Current classification                                               |
| -------------------------------------------------- | --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| HTTP control plane                                 | `smtp-api/cmd/api/main.go`; `/v1` routes in `smtp-api/internal/controlplane/routes.go` | Connects to PostgreSQL using `DATABASE_URL`. API keys, selected tenant/domain operations, submissions, and mailbox reads have PostgreSQL paths. A development `Store` remains available for development identities and process-local state; usage counters are in memory. Optional OIDC verifies configured tokens and provides browser sessions for pre-provisioned identities. Verified email/password signup, login, and recovery use Argon2id, PostgreSQL, expiring email tokens, and configured SMTP delivery. | **Partial, production-oriented paths**                               |
| SMTP ingress                                       | `smtp-session/src/main.rs`; `smtp-session/src/transport/server.rs`                                | Listens on loopback by default. Optional STARTTLS/implicit TLS and PostgreSQL-backed AUTH PLAIN are enabled explicitly; authenticated senders are restricted to verified domains owned by their tenant. `DATABASE_URL` attaches the queue, not the outbound worker. The PostgreSQL inbound acceptor is not wired into the executable.                                        | **Partial; secure submission, no production delivery**               |
| Queue persistence                                  | `smtp-queue/src/queue/store.rs`, `smtp-storage/postgres/delivery/000021_recipient_delivery_state.up.sql` | PostgreSQL queue, idempotency, per-recipient delivery/attempt state, domain claims, leases, and retries are implemented. Migration and recovery tests require an isolated PostgreSQL database.                                                                                                                                            | **Partial, durable storage; M05 validation in progress**             |
| Queue processing                                   | `smtp-delivery/src/main.rs`, `smtp-delivery/src/outbound/worker.rs`                               | The real worker claims domain batches, processes bounded SMTP delivery concurrently, records per-recipient results, and drains in-flight work. External delivery requires explicit opt-in and recipient-domain allowlisting.                                                                                                              | **Real worker wired; operational acceptance incomplete**             |
| Outbound SMTP                                      | `smtp-delivery/src/outbound/client.rs`, `smtp-delivery/src/outbound/processor.rs`, `smtp-delivery/src/outbound/dns.rs` | The client partitions by domain, validates and pins destination IPs, handles SMTP response classes and uncertain final responses, and integrates canonical-message validation. Destination/TLS policy and external interoperability requirements remain.                                                                                     | **Partial; M05 not production-ready**                                 |
| Inbound mail                                       | `smtp-session/src/inbound/mod.rs`                                                           | `PostgresInboundStore` can transactionally resolve and persist accepted messages. No current executable constructs or injects it. It rejects a transaction if recipients resolve to different mailbox IDs.                                                                                                                                                                   | **Reference implementation; not runtime-wired**                      |
| Mailbox storage and API                            | `smtp-core/src/mailbox/store/`; PostgreSQL-backed Go mailbox handlers                   | The Rust store exposes durable mailbox operations. The Go API exposes bounded PostgreSQL mailbox reads. The Rust store is not connected to a network mailbox protocol.                                                                                                                                                                                                       | **Partial; API reads are wired, Rust operations are not**            |
| IMAP and POP3                                      | `smtp-imap/src/lib.rs`, `smtp-pop3/src/lib.rs`                                          | Libraries contain parser/state-machine prototypes, with no network executables or database-backed authentication. IMAP LOGIN and POP3 PASS accept credentials without verification and return placeholder mailbox outcomes.                                                                                                                                                  | **Placeholder; not a usable service**                                |
| MIME and attachments                               | `smtp-core/src/message/mime.rs`, `smtp-core/src/attachments/`                           | MIME parsing has limits. Attachment scanning is injected and attachment content is held by an in-memory store. Neither is wired into the message pipeline or durable object storage.                                                                                                                                                                                         | **Reference-only**                                                   |
| Search, sync, rules, threading, auth protocols, AI | `smtp-core/src/`                                                                        | Domain primitives exist, including selected SQL mailbox operations. Most are not connected to a runtime workflow; several use in-memory state or injected interfaces.                                                                                                                                                                                                        | **Reference-only or partial; per-feature wiring required**           |
| Operations and infrastructure                      | `compose.yaml`, `Makefile`, `scripts/`, `.github/workflows/`                            | Compose starts PostgreSQL for local use. API health/readiness and process-local metrics exist. No production deployment target, object store, KMS, or centralized telemetry service is configured here.                                                                                                                                                                      | **Local development foundation; production infrastructure deferred** |

## Dashboard and CI wiring

The dashboard's typed control-plane client calls registered `/health/ready`
and `/v1` routes for API keys, domains, usage, submissions, and mailbox reads.
This confirms contract wiring, not that every operation is backed by a durable
production service.

The CI workflow runs Rust, Go, dashboard, and PostgreSQL migration/queue
integration jobs. The security workflow configures dependency review, Rust
advisory and Go vulnerability scans, secret scanning, and workflow checks. A
separate workflow validates Compose syntax; the release workflow builds Rust
binaries and publishes source/binary artifacts but does not deploy the
platform. These are configured checks; this local audit did not verify remote
workflow results. `make validate-foundation-config` passed locally.

### SMTP capability check at the M00 baseline

The session advertises `SIZE`, optional `8BITMIME`, and `AUTH PLAIN` when an
authenticator is configured. It does not advertise `STARTTLS`; issuing
`STARTTLS` returns 502. Although the synchronous command processor checks the
TLS requirement for AUTH, the live asynchronous session path handles AUTH
directly and does not make that check before challenging for or verifying
credentials. The live path can therefore process PLAIN credentials without
TLS when an authenticator is configured. Do not expose authenticated SMTP
submission until M01 fixes and tests this path. This is the recorded M00
baseline; the M01 follow-up below supersedes its runtime conclusions.

## Prioritized defect and security register

| Priority | Severity         | Finding and impact                                                                                                                                                                                                                    | Evidence                                                                                  |
| -------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| 1        | **High**         | The live SMTP AUTH path bypasses `require_tls_for_auth`, while EHLO can advertise PLAIN without TLS. With an authenticator configured, credentials may be sent and accepted over plaintext. STARTTLS is not implemented.              | `smtp-session/src/protocol/session.rs`, `smtp-session/src/runtime/config.rs`                               |
| 2        | **High**         | The runnable queue worker marks work successful using a fixed `TestProcessor::Success`; it does not call the SMTP delivery processor. A successful queue outcome is not evidence of remote SMTP acceptance or delivery.               | `smtp-queue/src/main.rs`, `smtp-queue/src/execution/processor.rs`, `smtp-delivery/src/outbound/processor.rs` |
| 3        | **High**         | Outbound delivery selects an MX destination from the first recipient, then sends all job recipients to that destination. Multi-domain jobs can be rejected or disclose other recipients and message content to the wrong domain's MX. | `smtp-delivery/src/outbound/client.rs`                                                             |
| 4        | **Medium**       | SMTP shutdown aborts all active session tasks instead of draining or safely terminating them; accepted/in-flight transactions can be interrupted.                                                                                     | `smtp-session/src/transport/server.rs`                                                              |
| 5        | **Medium**       | The inbound PostgreSQL acceptor rejects valid multi-recipient messages when the recipients resolve to different mailbox IDs, and the executable does not wire the acceptor.                                                           | `smtp-session/src/inbound/mod.rs`, `smtp-session/src/main.rs`                                 |
| 6        | **Medium**       | API usage counters and development identity state are process-local. Restarts or multiple API instances do not share these values, so they are not durable quota or distributed rate-limit enforcement.                               | `smtp-api/internal/controlplane/usage.go`, `smtp-api/cmd/api/main.go`                     |
| 7        | **Release gate** | IMAP/POP3 credential handling and mailbox responses are placeholders, and neither protocol has a real network listener. They must not be presented or deployed as mailbox services.                                                   | `smtp-imap/src/lib.rs`, `smtp-pop3/src/lib.rs`                                            |

The sequence in this register is the recommended implementation order at the
M00 baseline, subject to the milestone dependencies in the implementation
prompt. M01 addressed findings 1 and 4. Follow-up work removed the simulated
queue executable and added real delivery processing, addressing the core of
findings 2 and 3; M05's production acceptance remains open. Findings 5, 6,
and 7 remain open.

## Baseline

Baseline checks were run before M00 documentation changes.

| Command                                    | Result      | Notes                                                                                                                                                      |
| ------------------------------------------ | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `make doctor`                              | **Pass**    | Required toolchain and local prerequisites were present.                                                                                                   |
| `make fmt-check`                           | **Fail**    | Rust and Go formatting checks passed; Prettier reported existing formatting differences in `api-keys-view.tsx` and `control-plane-dashboard.tsx`.          |
| `make lint`                                | **Fail**    | `cargo clippy` stopped on existing warnings in `smtp-pop3`, `smtp-imap`, and an SMTP session test. The Make target did not reach Go vet or dashboard lint. |
| `cd smtp-api && go vet ./...`              | **Pass**    | Run separately after the Rust lint failure.                                                                                                                |
| `pnpm lint:dashboard`                      | **Pass**    | Run separately after the Rust lint failure.                                                                                                                |
| `make test`                                | **Pass**    | Rust workspace and Go tests passed. PostgreSQL integration tests requiring `TEST_DATABASE_URL` were ignored/not run.                                       |
| `make build`                               | **Pass**    | Rust workspace, Go API, and production dashboard build passed.                                                                                             |
| `make check`                               | **Not run** | It begins with the already-failing `make fmt-check`.                                                                                                       |
| `make test-queue`, `make migrate-validate` | **Not run** | No isolated disposable PostgreSQL test database was configured; these targets must not use a developer or production database.                             |

The baseline `make lint` findings are:

- `smtp-pop3/src/lib.rs`: `Session::new` has no `Default` implementation.
- `smtp-imap/src/lib.rs`: returns a temporary `let` binding directly.
- `smtp-session/src/protocol/session.rs` test: field assignment after `Config::default()`.

These are pre-existing baseline failures, not changes made by M00.

## M01 follow-up

M01 is complete for the authoritative specification and acceptance criteria:
certificate-backed STARTTLS/implicit TLS, TLS-only AUTH PLAIN, tenant-scoped
Argon2id credential verification and lifecycle APIs, verified-domain sender
authorization, shared authentication throttling, bounded connection handling,
certificate reload, and graceful shutdown are implemented. The network,
PostgreSQL, control-plane, and Python `smtplib` acceptance tests pass, including
a complete authenticated submission. Selecting and verifying a deployment's
secret-manager provider and expanding the client interoperability matrix remain
operational hardening, not unimplemented M01 code.
