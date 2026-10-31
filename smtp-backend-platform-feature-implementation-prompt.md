# SMTP Platform — Backend Feature Implementation Prompt

> **Purpose:** Use this document as the persistent implementation instruction for the `irfanalam13/smtp` repository. It is a repository-specific, feature-by-feature engineering prompt for evolving the current foundation into a secure, production-grade, multi-tenant email cloud platform.
>
> **Repository:** https://github.com/irfanalam13/smtp  
> **Primary branch inspected:** `main`  
> **Scope:** Backend and platform infrastructure first. Do not spend this implementation cycle redesigning the Next.js dashboard unless a backend API contract requires a coordinated frontend change.

---

## 1. Your role and execution instruction

Act as the principal backend engineer, distributed-systems architect, SMTP/MTA protocol engineer, database engineer, and application-security reviewer for this repository.

Your task is to **inspect, implement, test, and integrate** the next missing backend feature group. Do not merely describe an architecture or create placeholder interfaces.

At the beginning of every run:

1. Read this prompt completely.
2. Read `README.md`, `docs/README.md`, `docs/project-status.md`, `docs/phase-status.md`, `docs/phase-integration.md`, the relevant `docs/standards/` documents, and all relevant architecture ADRs.
3. Inspect the current code, tests, migrations, configuration, and service wiring for the feature being implemented. The repository may have changed since this prompt was written; current code and current documentation take precedence over assumptions in this file.
4. Run the relevant baseline checks before editing. Record existing failures separately from failures introduced by your changes.
5. Select the **first incomplete milestone in the dependency order in Section 8**, unless the user explicitly names a milestone ID. Implement only that milestone in the current run. Do not attempt to build the entire email cloud in one oversized change.
6. Implement a complete vertical slice: domain model, persistence/migration, service integration, API/protocol integration where applicable, configuration, tests, observability, and documentation.
7. Run the relevant validation commands and fix regressions before declaring the milestone complete.
8. Update `docs/phase-status.md`, `docs/project-status.md`, the affected standards/architecture documentation, and ADRs when a material architectural decision changes.

**Do not stop after the audit.** Unless blocked by missing credentials, unavailable infrastructure, an ambiguous product decision, or a real external dependency, implement the selected feature now. Do not ask for confirmation for ordinary engineering decisions; make a conservative, reversible decision, document it, and continue.

### Invocation examples

Use this prompt with an implementation agent and ask it to:

```text
Read smtp-backend-platform-feature-implementation-prompt.md and implement the next incomplete milestone in dependency order. Inspect the current repository first, make the code changes, add migrations and tests, run validation, and update the project status documents. Do not only provide a plan.
```

Or select one explicit feature:

```text
Read smtp-backend-platform-feature-implementation-prompt.md and implement milestone M05 only. Verify prerequisites, implement it end to end, run tests, and update documentation.
```

---

## 2. Product objective

Build a professional email cloud platform with the core capabilities expected of a modern hosted email and transactional email provider:

- A standards-based SMTP message transfer and submission system.
- Reliable inbound reception and outbound delivery.
- Durable, tenant-isolated message queues and mailbox storage.
- Domain ownership verification and sender authentication.
- Secure mailbox access over IMAP, with POP3 only if retained in the product scope.
- A multi-tenant control plane, APIs, API keys, users, teams, permissions, quotas, and audit history.
- Deliverability and abuse prevention: SPF, DKIM, DMARC, bounce handling, suppression, reputation signals, and tenant/domain limits.
- Search, threading, attachments, mailbox rules, synchronization, and retention.
- Webhooks, event delivery, analytics, monitoring, operational controls, and disaster recovery.
- Optional AI capabilities only after data security, consent, and the deterministic mail pipeline are stable.

The target is a dependable email platform, not a demo that merely accepts TCP connections or returns successful SMTP replies. The system must distinguish every lifecycle boundary accurately: **accepted by this service**, **persisted**, **queued**, **attempted**, **accepted by a remote SMTP server**, **bounced**, and **delivered to a recipient mailbox**. Never claim inbox placement based only on a remote SMTP `250` response.

---

## 3. Repository baseline: what exists and what it does not mean

The repository already contains meaningful backend foundations. The main engineering problem is not simply the number of files; it is that several implementations are bounded prototypes, isolated domain primitives, or not yet connected into end-to-end production services. Preserve and extend useful work instead of replacing it blindly.

### 3.1 Current technology and service boundaries

Keep the current stack unless a measured technical reason and an ADR justify a change:

- **Rust workspace:** SMTP ingress/session, queue, outbound delivery, shared mail and mailbox domain logic, IMAP and POP3 prototypes.
- **Go:** HTTP control-plane API, tenant/domain/API-key/OIDC handlers, PostgreSQL repositories, health and API observability.
- **PostgreSQL:** durable control-plane, queue, inbound-message and mailbox foundations, with versioned SQL migrations.
- **Next.js / TypeScript:** dashboard under `smtp-dashboard/`; keep backend contracts stable and typed.
- **Docker Compose / Make:** local development, PostgreSQL and validation workflows.

The root `Cargo.toml` currently includes `smtp-session`, `smtp-queue`, `smtp-delivery`, `smtp-core`, `smtp-imap`, and `smtp-pop3` as Rust workspace members. The Go API is a separate module under `smtp-api/`.

### 3.2 Important existing code locations

| Area | Existing paths | Current interpretation |
|---|---|---|
| SMTP ingress | `smtp-session/src/transport/server.rs`, `session.rs`, `command.rs`, `config.rs`, `inbound.rs` | A working bounded SMTP session foundation with explicit states, DATA framing, optional implicit TLS configuration, injectable AUTH, queue and inbound hooks. It is not yet a complete public SMTP submission/MTA implementation. |
| Queue | `smtp-queue/src/queue/store.rs`, `model.rs`, `worker.rs`, `processor.rs`, `retry.rs` | PostgreSQL enqueue/idempotency, leases, retries and attempt records exist. Worker and outcome semantics still need production hardening. |
| Outbound delivery | `smtp-delivery/src/outbound/client.rs`, `dns.rs`, `processor.rs`, `config.rs`, `outcome.rs` | Bounded MX lookup, destination filtering, SMTP dialogue and STARTTLS client support exist. Multi-recipient/domain handling and durable per-recipient delivery outcomes need work. |
| Shared backend core | `smtp-core/src/` | Includes MIME, mailbox, authentication, attachment, search, threading, rules, synchronization, regional-topology and AI domain primitives. Some are in-memory/reference implementations and are not integrated production services. |
| Mailbox persistence | `smtp-core/src/mailbox_store.rs` and `smtp-core/src/mailbox_store/` | PostgreSQL mailbox operations and workflow modules exist. Wire them into real service/protocol paths and verify authorization and consistency. |
| IMAP | `smtp-imap/src/lib.rs` | Parser/state-machine prototype only; not a real IMAP network service backed by mailboxes. |
| POP3 | `smtp-pop3/src/lib.rs` | Parser/state-machine prototype only; not a real POP3 network service backed by mailboxes. |
| HTTP API | `smtp-api/cmd/api/`, `smtp-api/internal/controlplane/` | Go API with OIDC/API-key foundations, tenant/domain management, send endpoint, mailbox metadata reads, and PostgreSQL repositories. Several quota/rate-limit/read-model paths are still development-oriented or incomplete. |
| Database | `smtp-storage/postgres/` | Versioned migrations through the current mailbox and workflow foundations. Extend migrations without rewriting deployed history. |
| Dashboard | `smtp-dashboard/src/` | Next.js dashboard organized by features. Only make frontend changes required to consume completed backend features. |
| Documentation | `docs/architecture/`, `docs/standards/`, `docs/api/`, `docs/phase-status.md` | Existing scope and design records. These are important inputs, not documentation to discard. |

### 3.3 Directly observed gaps to account for

Treat the following as audit findings to verify against the current checkout before making changes:

1. **SMTP STARTTLS is not implemented on ingress.** `smtp-session/src/protocol/session.rs` currently responds to `STARTTLS` with `502`; `smtp-session/src/transport/server.rs` supports implicit TLS configuration but does not upgrade an established plaintext connection to TLS.
2. **Review and fix the SMTP AUTH TLS enforcement path.** `Session::run` intercepts `Command::Auth` and calls the asynchronous `authenticate` method instead of routing it through `process`. The asynchronous method must enforce `require_tls_for_auth` before reading or validating credentials. EHLO must not advertise AUTH on a plaintext connection when policy requires TLS. Add a regression test proving credentials are never accepted or challenged in plaintext under the secure policy.
3. **Queue outcome names are test-oriented.** `mail-queue` currently uses `TestProcessorOutcome` and a `simulated_succeeded` terminal state. `SmtpProcessor` must not report a real remote SMTP acceptance as a simulated test success. Introduce explicit production delivery outcomes and migrations while preserving historical state compatibility.
4. **Worker reliability needs hardening.** The current worker processes one job at a time, has no explicit lease heartbeat/extension, and a processor error can exit the worker loop before the claimed lease is completed or safely released. Define cancellation, lease renewal, bounded concurrency, graceful shutdown, and at-least-once retry semantics.
5. **Outbound SMTP must be recipient/domain aware.** `smtp-delivery/src/outbound/client.rs` derives the MX route from the first recipient and then issues RCPT commands for all recipients. Partition recipients by destination domain and persist each recipient's accepted/rejected/retry/permanent-failure result independently. Do not lose RCPT-level failures when another recipient succeeds.
6. **Inbound multi-recipient routing is constrained.** `PostgresInboundStore::accept` currently rejects a transaction when recipients resolve to different mailbox IDs. Design safe per-mailbox fan-out so a valid SMTP transaction can address more than one local mailbox without cross-tenant leakage or partial-commit ambiguity.
7. **IMAP and POP3 are not production protocol servers.** The IMAP session accepts LOGIN without a real authenticator and returns placeholder command success. The POP3 session accepts a password without verification and returns zero-message placeholders. They must never be exposed as real mailbox services until authentication, network/TLS, storage, and protocol behavior are implemented.
8. **Mailbox code is not fully integrated.** The Rust mailbox store contains substantial PostgreSQL domain operations, while the Go mailbox API primarily exposes authorized mailbox/message metadata reads. Define one clear service boundary and connect folder/message/flag/label/attachment operations to it. Avoid implementing the same business invariants independently in Go and Rust.
9. **Usage and throttling are not yet production distributed controls.** `smtp-api/internal/controlplane/usage.go` uses in-memory counters. The API principal path also uses in-memory store rate limiting. Persist metering and use a distributed, tenant-aware enforcement design before multi-instance deployment.
10. **Attachment storage is a reference implementation.** `smtp-core/src/attachments.rs` encrypts, compresses, and deduplicates content in in-memory maps. Add a durable object-storage adapter, metadata persistence, key management, scanning integration, and lifecycle policies before claiming production attachment storage.
11. **Search and other mailbox capabilities are partly reference-level.** `smtp-core/src/search.rs` includes an in-memory `SearchIndex` as well as a PostgreSQL search function. Threading, synchronization, and AI modules contain reusable primitives but require durable adapters, authorization, indexing jobs, and service integration.
12. **Email authentication code is not protocol-complete.** `smtp-core/src/authentication.rs` contains bounded SPF evaluation and injectable DKIM signing/verifying interfaces, not a complete standards-compliant SPF evaluator, DNS-backed key lifecycle, or complete DKIM implementation. DMARC alignment is simplified; ARC and BIMI modules are validation foundations, not complete interoperable implementations. Do not promote these primitives to enforcement without RFC conformance tests and vetted cryptographic implementations.
13. **Regional topology code is not multi-region infrastructure.** `smtp-core/src/regional_topology.rs` validates a topology model; it does not provide geo-routing, replicated state, failover, or disaster recovery.
14. **Several future service directories are documentation-only.** At the time of inspection, directories such as `smtp-ai/`, `smtp-analytics/`, `smtp-auth/`, `smtp-cli/`, `smtp-kubernetes/`, `smtp-monitoring/`, `smtp-parser/`, `smtp-sdk/`, and `smtp-security/` contained README placeholders rather than complete services/SDKs. Recheck current tree before implementing; create real implementations only when their dependency milestone is reached.
15. **Production readiness is explicitly not claimed by the project itself.** Root `README.md` and `docs/phase-status.md` describe the repository as a development/validation foundation. Preserve this honest status until all production exit criteria are met.

These findings are not permission to rewrite everything. First confirm each issue in the current checkout, add focused regression coverage, and fix the underlying behavior in the correct package.

---

## 4. Non-negotiable engineering principles

### 4.1 Correctness before feature count

- Never add a feature that only returns a success response without executing the real operation.
- Never advertise SMTP/IMAP/POP3 capabilities that are not fully supported by the active server configuration.
- Never map test simulation, local queue acceptance, remote SMTP acceptance, or mailbox delivery to the same status.
- Do not silently swallow errors. Classify them as transient, permanent, policy/security, configuration, or internal failures.
- Design for at-least-once job execution. Use idempotency and recipient-specific state to make retries safe; do not promise exactly-once remote email delivery.

### 4.2 Tenant isolation and authorization

- Every tenant-owned query, update, insert, list, background job and object-storage operation must be scoped to a verified tenant and an authorized principal.
- Never trust a tenant ID, mailbox ID, sender address, domain ID, or object key merely because it came from a request body.
- Enforce tenant ownership in the service layer and database constraints where practical. Do not rely exclusively on frontend visibility or handler checks.
- Separate human-user identity from service/API-key identity. Apply scopes and mailbox permissions for every operation.
- Add cross-tenant negative tests for each new resource and API route.

### 4.3 Security, privacy and abuse prevention

- Never log message bodies, attachment bytes, passwords, OAuth tokens, API-key secrets, DKIM private keys, SMTP AUTH payloads, or raw credentials.
- Keep logs structured and redact sensitive values. Do not include raw untrusted SMTP commands in log messages without strict sanitization and truncation.
- Use constant-time comparisons for secret verifiers and cryptographic authentication checks.
- Use modern password hashing (Argon2id or another approved memory-hard password KDF) with per-record salts and versioned parameters. Never store plaintext SMTP passwords.
- Use a cryptographically secure random number generator for tokens, reset links, nonces, and secret material. Use a proper KMS/HSM or secret-manager integration for production keys.
- Prevent open-relay behavior. Outbound submission must require valid authentication, authorized sender domains, tenant policy, quotas, and recipient limits. Inbound MTA acceptance must be limited to configured local domains/recipients unless a separately reviewed relay policy applies.
- Protect against DNS rebinding and SSRF. Validate every resolved address, pin the validated address to the actual socket connection, and re-check redirects/proxy behavior if used.
- Apply bounded timeouts, bounded allocations, bounded concurrency, request limits, SMTP line limits, MIME recursion limits, attachment decompression limits, and rate limits.
- Provide retention, deletion, legal hold, backup encryption, audit, and data export semantics before offering hosted mailbox storage.

### 4.4 Database and event integrity

- Use PostgreSQL transactions for multi-row mail acceptance and mailbox mutations.
- Keep migrations immutable once deployed. Add a new numbered migration for schema evolution; do not edit old migration files to retrofit a production schema.
- Add correct foreign keys, uniqueness constraints, tenant-scoped indexes, status checks, and query indexes.
- Avoid storing large attachments in ordinary relational rows unless a measured requirement justifies it. Use a storage abstraction and durable object storage for large blobs.
- Use a transactional outbox for reliable events. Publish asynchronously with idempotency and retries; never publish an event before its database transaction commits.
- Design pagination as stable cursor pagination for large/high-churn datasets. Avoid unbounded result sets.

### 4.5 Code quality and architecture

- Preserve the Rust data-plane and Go control-plane split unless an ADR justifies changing it.
- Avoid circular dependencies and duplicated business rules across services.
- Keep pure domain logic separate from transport, persistence, cloud providers, and framework code.
- Use typed errors and explicit state transitions.
- Use traits/interfaces at real integration seams (DNS, KMS, object storage, malware scanner, event broker, identity provider); do not create unused abstractions.
- Pin and audit dependencies. Prefer actively maintained libraries with security support. Review advisories and licenses.
- Make feature flags explicit and secure-by-default. A disabled feature must fail closed and be clearly reported in health/readiness diagnostics.

---

## 5. Target backend architecture

Maintain clear logical boundaries. These can be separate binaries or deployable services as deployment needs mature; do not split into microservices merely to create more folders.

### 5.1 Data plane

**Rust services/libraries** should own high-throughput mail transport and mailbox operations:

- SMTP inbound MTA and authenticated submission listener.
- Message parsing and MIME normalization.
- Durable enqueue and delivery workers.
- MX/DNS routing, outbound SMTP, TLS policy and delivery state.
- Inbound local-domain routing and mailbox persistence.
- IMAP server; POP3 only if deliberately retained.
- Shared message, mailbox, authentication and policy domain logic.

### 5.2 Control plane

**Go API** should own product and tenant management:

- OIDC/resource-server integration, user/tenant membership, API keys and scoped authorization.
- Domain registration, DNS verification, DKIM configuration and domain readiness.
- Mailbox provisioning/administration through a single documented mailbox service/repository boundary.
- Sending API, message activity, recipient status and delivery analytics.
- Quotas, usage metering, plan entitlements, audit, webhooks and account settings.
- Public API compatibility, versioning, request validation and OpenAPI contract.

### 5.3 Persistence and infrastructure

- PostgreSQL for transactional metadata, tenant/control-plane data, queue state, mailbox metadata, delivery events, idempotency, audit/outbox and usage metering.
- S3-compatible object storage (or another selected durable blob store) for raw message blobs and attachments when required by size/retention profile.
- Redis only for an approved need such as distributed rate limits, cache, or ephemeral coordination. Do not add it merely because the platform is SaaS.
- A real secret manager/KMS for API-key material, SMTP credentials, DKIM keys, data-encryption keys, and certificate lifecycle.
- OpenTelemetry-compatible metrics, logs and tracing, with bounded-cardinality labels and privacy-safe telemetry.
- Kubernetes/containers only after service health, graceful shutdown, resource limits, probes, network policy, secret injection, migrations and rollback behavior are defined.

### 5.4 Service communication

Use direct libraries for same-process calls. Use a documented, authenticated internal API or message/event contract when a real process boundary is required. Introduce gRPC, Kafka, NATS, Redis streams, or another broker only when a concrete throughput, isolation, or delivery guarantee requires it. Do not build a fake broker or unnecessary RPC framework.

---

## 6. Required implementation contract for every milestone

Every milestone must produce all applicable deliverables below.

### A. Discovery and design

- Identify existing implementation and all callers.
- Identify standards/RFC requirements and supported feature profile.
- Document state transitions, failure semantics, authorization rules, capacity bounds, and threat model.
- Write/update an ADR if the change affects service boundaries, storage format, queue semantics, security model, or an external dependency.

### B. Implementation

- Implement real domain logic and a production integration path.
- Add configuration with validation and secure defaults.
- Add database migrations and indexes if persistence changes.
- Add authorization and tenant filters at every required boundary.
- Add idempotency, retry, timeout, cancellation, and concurrency behavior appropriate to the feature.
- Wire the implementation into a real listener, API route, worker, or scheduled job. A library that is never called is not a completed feature.
- Update OpenAPI/SDK contracts when a public API changes.

### C. Tests

Add appropriate layers of tests:

1. Unit tests for parsers, state transitions, validation, policy and error classification.
2. Property/fuzz tests for untrusted protocol input, MIME, binary framing, headers and identifiers.
3. Database integration tests against an isolated PostgreSQL database, including rollback and concurrent access where relevant.
4. Network integration tests with deterministic SMTP/IMAP/POP3 fixtures and TLS handshakes.
5. Cross-tenant and privilege-escalation negative tests.
6. Failure/recovery tests: worker crash, duplicate enqueue, lease expiry, process restart, DB unavailable, DNS timeout, object-store failure, duplicate webhook, and graceful shutdown.
7. Performance/load tests with explicit workload, resource bounds, latency and throughput measurements for high-volume paths.

Never use production credentials, public recipient addresses, or a live external SMTP server as a default test dependency.

### D. Observability and operations

- Add metrics for accepted/rejected messages, queue depth/age, delivery attempts, remote response classes, retries, bounces, auth failures, TLS failures, DNS failures, mailbox storage, attachment scanning and API errors.
- Add structured tracing across request/message/tenant/attempt identifiers. Do not use email addresses, message subjects, raw message IDs from users, or tenant IDs as unbounded metric labels.
- Add health/readiness checks for required dependencies and explicit disabled-feature diagnostics.
- Document capacity controls, configuration, runbooks, migration order, backfill/reindex procedures, retention and rollback/recovery.

### E. Definition of done

A milestone is complete only when:

- Its real runtime path is wired and tested.
- It has no fake/stub behavior represented as production success.
- Security and tenant isolation tests pass.
- Relevant format, lint, test and build commands pass.
- New migration(s) are validated on a clean database and an upgrade database.
- Documentation accurately describes implemented behavior and limitations.
- The status register marks the milestone complete only to the tested scope. Do not mark broader phases complete merely because a subset was implemented.

---

## 7. Repository-specific validation commands

First inspect `make help`, scripts, toolchain files and CI workflows. Use the repository's supported commands. The following targets are present in the current Makefile and are the normal validation entry points:

```bash
make doctor
make fmt-check
make lint
make test
make build
make check
```

Focused validation:

```bash
cargo test -p smtp-ingress
cargo test -p mail-queue
cargo test -p mail-delivery
cargo test -p shared-types
cargo test -p smtp-imap
cargo test -p smtp-pop3

cd smtp-api && go test ./... && go vet ./...

pnpm lint:dashboard
pnpm build:dashboard
```

PostgreSQL integration/migration validation must use a disposable isolated database, never a developer's only database or a production database:

```bash
make migrate-validate
make test-queue
```

Run tests with the repository's actual environment and versions. If a required external integration is unavailable, test it through a deterministic fake and report the unexecuted live verification explicitly. Do not fabricate passing test results.

---

## 8. Feature-by-feature implementation roadmap

Implement in dependency order. Some milestones can be parallelized only after their contracts are stable. The order below is intentional: protocol correctness and durable delivery precede optional mailbox UX, AI and global scale.

### M00 — Re-audit, baseline, integration map and gap register

**Objective:** Reconcile this prompt with the current checkout and establish a trustworthy engineering baseline.

**Tasks:**

- Inspect all Rust, Go, SQL, scripts, Docker/Compose, CI, dashboard API clients and docs.
- Map each service's executable entry point, library API, persistence adapter, configuration source and production caller.
- Identify `TODO`, `todo!`, `unimplemented!`, placeholder return values, test-only branches, in-memory-only stores, dead code and features advertised but not wired.
- Run the baseline checks and document pre-existing failures.
- Produce/update a service integration matrix and prioritized defect/security register.
- Verify all advertised SMTP capabilities against actual runtime support.

**Acceptance:** Every core backend feature is classified as production-wired, partial/reference-only, placeholder, documentation-only, or intentionally deferred. Baseline results are recorded without claiming the system is production-ready.

### M01 — SMTP transport security, STARTTLS and authenticated submission

**Objective:** Make SMTP connections secure and prevent credential disclosure or unauthorized relaying.

**Tasks:**

- Implement in-place STARTTLS negotiation for inbound SMTP using the configured Rustls server configuration. Correctly replace the transport after TLS negotiation, reset EHLO/transaction state as required, and prevent protocol-command injection across the upgrade boundary.
- Retain implicit TLS only where the deployment profile requires it; document port/listener expectations.
- Ensure EHLO advertises STARTTLS only when configured and available. Do not advertise AUTH mechanisms unless they are enabled and the transport policy permits their use.
- Fix and regression-test the asynchronous AUTH path so TLS requirements are enforced before any credential challenge or authentication attempt.
- Implement a real credential verifier backed by tenant-scoped credential records. Store password hashes only; support versioned hash parameters and secure rotation/revocation.
- Support only reviewed SASL mechanisms initially. Add mechanisms such as PLAIN only inside TLS; add OAuth SASL only with a validated token audience, issuer, expiry, tenant and authorization scope.
- Add per-IP, per-account and per-tenant authentication throttling, lockout/backoff controls, connection limits, command deadlines and abuse telemetry.
- Ensure TLS errors do not silently downgrade the connection.
- Implement graceful listener shutdown: stop accepting new connections, allow bounded in-flight transactions to finish or safely terminate, then close remaining sessions.

**Tests:** plaintext AUTH is rejected when TLS is required; no plaintext AUTH advertisement/challenge; valid TLS AUTH succeeds; invalid/revoked/wrong-tenant credentials fail; STARTTLS success/failure and state reset; connection exhaustion; slow-client timeout; shutdown with active clients.

**Acceptance:** A real SMTP client can complete a TLS-secured authenticated submission transaction. No open relay is possible through the public submission listener.

### M02 — SMTP protocol conformance and extension profile

**Objective:** Complete and test a clearly defined SMTP/ESMTP profile.

**Tasks:**

- Review the SMTP state machine and parser against the selected RFC 5321/ESMTP profile. Validate command sequencing, path syntax, null reverse-path, reply classes, multiline replies, strict CRLF behavior, DATA dot-unstuffing and transaction reset.
- Make EHLO capability advertisement a function of actual runtime configuration and complete implementation, not a static list.
- Correctly enforce the declared SIZE maximum before and during message transfer. Add per-tenant and per-domain limits.
- Add supported `8BITMIME` behavior and tests.
- Add SMTPUTF8 only after Unicode local-part storage, IDNA domain handling, normalization, queue, DNS and outbound client support are designed end to end.
- Add DSN envelope support (`ENVID`, `RET`, `NOTIFY`, `ORCPT`) only with durable recipient-level delivery records and RFC-compatible report generation.
- Consider PIPELINING, CHUNKING/BDAT and BINARYMIME as separate explicitly gated extensions. Implement safe ordering, streaming and backpressure before advertising them.
- Parse duplicate/unknown/malformed ESMTP parameters deterministically and return appropriate reply codes.
- Use bounded parsers and property/fuzz testing for malformed, oversized and adversarial input.

**Acceptance:** The server advertises only fully implemented capabilities. A conformance fixture suite covers supported commands, reply codes, transitions, error recovery and extension behavior.

### M03 — Canonical message model, RFC 5322 and MIME pipeline

**Objective:** Establish one safe representation for envelope metadata, exact raw message bytes, parsed headers, body parts and attachments.

**Tasks:**

- Define versioned canonical message/envelope models. Keep SMTP envelope sender/recipients separate from RFC 5322 From/To/Cc/Message-ID headers.
- Preserve exact accepted raw bytes where retention/legal requirements require it; parse into bounded indexed metadata without corrupting original content.
- Extend and integrate `smtp-core/src/mime.rs` using the existing `mail-parser` dependency. Verify header folding, encoded words, nested multipart messages, content-transfer-encoding, RFC 2231 filenames, inline parts and malformed MIME behavior.
- Set limits for header count/bytes, MIME depth, part count, attachment count, decoded bytes, compression ratios and parsing CPU/time.
- Normalize line endings only in generated messages or a documented delivery transformation; do not mutate the canonical stored original without recording the transformation.
- Prevent header injection in generated From/To/Subject/Message-ID/Received/DKIM fields.
- Add SMTPUTF8/IDNA data-model support only as an end-to-end feature, not a parser-only flag.
- Connect MIME parsing to the queue and inbound storage pipeline and persist metadata required for search and mailbox rendering.

**Acceptance:** Real inbound and outbound messages use the canonical pipeline. MIME limits are enforced without unbounded memory allocation, and parsing cannot cross tenant/security boundaries.

### M04 — Production durable queue and worker lifecycle

**Objective:** Turn the existing PostgreSQL queue into a reliable at-least-once work system with explicit production states.

**Tasks:**

- Preserve the existing idempotent enqueue, PostgreSQL transaction and `FOR UPDATE SKIP LOCKED` foundation.
- Separate test/simulated processor types from production delivery processor types. Introduce explicit states such as `queued`, `leased/processing`, `retry_scheduled`, `accepted_by_remote`, `partially_accepted`, `permanently_failed`, `cancelled`, and `suppressed` as justified by the recipient-level model. Preserve historical states and provide safe migrations/read compatibility.
- Model an individual delivery per recipient (or a rigorously defined grouped job) so different destination domains, remote responses, retries and bounces can be represented independently.
- Persist attempt start time when work begins and finish time when it ends. Store safe, bounded diagnostics and SMTP response metadata without credentials or raw message content.
- Add lease heartbeat/extension and lease fencing. A worker must not complete a job after losing its lease.
- Add configurable worker concurrency, bounded in-flight bytes, per-domain concurrency, fairness and graceful cancellation.
- A processor failure must be classified and recorded; it must not unexpectedly terminate the entire worker loop or leave a job permanently leased.
- Implement exponential backoff with jitter, maximum attempts, maximum age, permanent/transient SMTP response classification and dead-letter inspection/replay workflows.
- Add crash/restart, duplicate, concurrent claim, lease-expiry and lease-loss tests.
- Keep queue acceptance separate from delivery status in APIs and metrics.

**Acceptance:** The queue safely recovers from worker termination and DB/network failures. Repeated processing is expected and safe. No test-only success state is used for real delivery.

### M05 — Outbound MTA delivery and per-recipient state

**Objective:** Deliver messages safely and reliably to external recipient domains.

**Tasks:**

- Build on `smtp-delivery/src/outbound/client.rs` and `dns.rs`; retain bounded DNS and validated socket candidates.
- Partition recipients by normalized destination domain. Resolve and connect to the correct MX route for each domain; never route all recipients through the first recipient's MX.
- Pin the validated resolved IP address to the actual TCP connection to prevent DNS rebinding/TOCTOU gaps. Expand destination policy to cover all relevant non-public, special-use, link-local, private, multicast, reserved and mapped address ranges for IPv4 and IPv6.
- Implement MX fallback ordering, null MX handling, DNS negative results, implicit MX behavior where applicable, connection limits and bounded DNS caching.
- Implement STARTTLS with certificate/hostname verification. Make opportunistic vs required TLS a tenant/domain policy with secure defaults; never silently misreport a TLS-required failure as successful cleartext delivery.
- Add MTA-STS and TLS reporting only with their complete fetch/cache/policy/validation/reporting semantics. Add DANE only with DNSSEC validation and a reviewed trust model. Do not implement a superficial TXT check and call it complete.
- Send MAIL FROM/RCPT TO/DATA using validated paths and bounded commands/replies. Correctly handle multiline responses, connection drops, 4xx/5xx, ambiguous final DATA results and QUIT.
- Persist every recipient's remote response, attempt, next retry, final status and remote server metadata. If some recipients succeed and some fail, retain each outcome and retry only eligible recipients.
- Add optional authenticated smarthost relay support only as a separately configured mode with protected credentials and explicit TLS policy. Direct-to-MX MTA mode must not accidentally send SMTP AUTH credentials.
- Add per-domain/IP throttling, concurrency controls, connection reuse policy, destination backoff, circuit breakers and safe DNS caching.
- Make the public status clear: remote SMTP acceptance is not a guarantee of inbox placement.

**Acceptance:** Multi-domain batches are routed correctly; mixed recipient outcomes are durable and recoverable; no unsafe destination is contacted; transport security policy is enforced and observable.

### M06 — Inbound MTA, local routing and durable acceptance

**Objective:** Accept mail for verified tenant domains and persist it safely before returning SMTP success.

**Tasks:**

- Extend `smtp-session/src/inbound/mod.rs` and `PostgresInboundStore` into a production inbound pipeline with tenant/domain/mailbox authorization.
- Route verified domains and active primary/alias addresses to the correct mailbox. Reject unknown recipients according to policy; do not become an open relay.
- Implement atomic multi-recipient fan-out. A single SMTP transaction may target multiple local mailboxes (including recipients across tenants only if the product policy explicitly permits it); all acceptance outcomes must be consistent and tenant isolated. Define whether the platform commits all local recipients or explicitly reports partial recipient acceptance before DATA.
- Persist raw message content and envelope/Received metadata transactionally or through a durable spool plus transactional metadata handoff. Return SMTP `250` only after the configured durable acceptance boundary has succeeded.
- Add duplicate detection with collision-safe idempotency and explicit retry semantics. A content hash may help deduplication but must not merge distinct messages incorrectly.
- Integrate MIME parsing, spam/virus scanning, sender authentication and tenant policy as ordered pipeline stages. Define temporary vs permanent SMTP replies for scanner/downstream failures.
- Add connection/IP/domain rate limits, recipient enumeration protection, message-size/quota enforcement, greylisting only if the product chooses it, and abuse telemetry.
- Add retry-safe storage/outbox jobs for downstream indexing, notifications, rules and events.

**Acceptance:** Inbound mail to verified mailboxes is durably stored and correctly fanned out. Invalid recipients are rejected without cross-tenant disclosure. SMTP success always corresponds to the documented durable acceptance boundary.

### M07 — DNS, SPF, DKIM, DMARC and email authentication

**Objective:** Implement real standards-based domain and message authentication rather than simplified reference checks.

**Tasks:**

- Build bounded DNS resolver adapters with TTL-aware caching, negative caching, timeouts, CNAME handling, DNSSEC status where required, and per-message lookup budgets.
- Implement SPF evaluation per RFC 7208, including all supported mechanisms/modifiers, include/redirect recursion, lookup limits, void lookups and correct `permerror`/`temperror` behavior. Never use the simplified evaluator as an enforcement engine.
- Implement full DKIM header/body canonicalization, signature parsing, signed-header selection, body length handling, RSA-SHA256 and/or Ed25519-SHA256 according to the supported profile, selector lookup, key validation, and robust verification. Use vetted cryptographic libraries.
- Implement outbound DKIM signing with protected private keys, tenant domain/selector configuration, key rotation, DNS publication instructions, selector overlap and revocation procedures. Private keys must never enter logs or ordinary database columns unencrypted.
- Implement DMARC record discovery, organizational-domain alignment (including a maintained Public Suffix List approach), strict/relaxed alignment, `p`/`sp`/`np`, percentage, report URI policy, and correct dispositions.
- Integrate authentication results with inbound trust, quarantine, rejection and outbound delivery policy. Preserve raw evidence and expose safe summarized results.
- Implement aggregate DMARC report generation/ingestion and privacy-safe reporting. Treat forensic reporting as sensitive and opt-in.
- Extend ARC and BIMI only after SPF/DKIM/DMARC behavior is conformant. Validate ARC cryptographic chain and header semantics; validate BIMI SVG/certificate/HTTPS and provider eligibility before making claims.
- Consider MTA-STS, TLS-RPT, DANE and REQUIRETLS only with complete reviewed implementations and explicit product support policy.

**Acceptance:** RFC-indexed test vectors, valid/invalid key/signature fixtures, DNS timeout and lookup-exhaustion tests, and real fixture domains demonstrate correct results. Enforcement cannot be enabled for an incomplete evaluator.

### M08 — Mailbox storage integration and administration

**Objective:** Connect the existing PostgreSQL mailbox store to real tenant and mailbox services.

**Tasks:**

- Audit `smtp-core/src/mailbox_store.rs` and all submodules before extending them. Reuse existing mailbox creation, folder creation, message storage, flags, labels, rules, submission, retention and outbox logic where correct.
- Define the single owner of mailbox business invariants. Keep Go control-plane management and Rust mailbox data-plane responsibilities explicit; avoid duplicated rules across Go and Rust.
- Wire mailbox provisioning, ownership, aliases, shared mailboxes, memberships and permissions into authenticated service/API operations.
- Implement folder listing/creation/rename/delete, special-use folders, UIDVALIDITY/UIDNEXT allocation, message append/copy/move/delete, expunge, flags and labels with correct transaction semantics.
- Add message list/detail/body/header retrieval endpoints with mailbox-scoped authorization, stable pagination, range/stream handling and content access auditing.
- Enforce mailbox quota under concurrency. Ensure `used_bytes` is correct under insert/delete/move/retention/restore and cannot exceed quota due to races.
- Add per-mailbox access policy checks (owner, read, read-write, manage), membership revocation and shared-mailbox behavior.
- Connect inbound routing and outbound sent-copy/drafts workflows to the same mailbox data model.
- Add consistency checks for mailbox UID allocation, folder trees, aliases, message counts, quota accounting and change-journal state.

**Acceptance:** Mailboxes can be provisioned and operated through real APIs/services; messages arrive in the correct folders; authorization and quota invariants survive concurrency and restart.

### M09 — Production IMAP server

**Objective:** Replace the current IMAP state-machine demo with a real, authenticated, TLS-protected mailbox server.

**Tasks:**

- Audit `smtp-imap/src/lib.rs`; do not expose its placeholder LOGIN or command-accepted behavior as a real server.
- Implement a bounded streaming IMAP parser that correctly handles tags, atoms, quoted strings, literals, synchronizing/non-synchronizing literals if supported, continuation responses, invalid UTF-8 and command pipelining.
- Implement a TCP listener, TLS/STARTTLS or implicit TLS according to the selected profile, connection limits, timeouts, graceful shutdown and structured observability.
- Integrate the shared identity and mailbox authorization boundary. Password/OAuth verification must reuse secure credential and tenant policies; never accept any non-empty password as authentication.
- Implement a documented IMAP4rev2 baseline: CAPABILITY, NOOP, LOGOUT, LOGIN/AUTHENTICATE, NAMESPACE, LIST/LSUB, STATUS, SELECT/EXAMINE, FETCH, STORE, SEARCH, COPY, MOVE, APPEND, CREATE/DELETE/RENAME, UID variants, EXPUNGE and IDLE where selected. Add extensions only when fully supported.
- Correctly implement sequence numbers, UIDs, UIDVALIDITY, UIDNEXT, message flags, mod-sequences/CONDSTORE/QRESYNC if included, literals, partial fetches and unsolicited state changes.
- Implement concurrent mailbox access and synchronization using the durable change journal. Ensure UID allocation and flags are transactionally correct.
- Return standards-compliant tagged/untagged responses and error codes. Do not advertise IMAP4rev2 or an extension until its required behavior is supported.
- Add interoperability tests with at least two independent IMAP clients/libraries, plus fuzzing and large-literal tests.

**Acceptance:** Users can securely connect with a real IMAP client, list and select mailboxes, read and mutate messages, and observe consistent state across concurrent clients and reconnects.

### M10 — POP3 server (only if product scope requires it)

**Objective:** Provide a standards-based POP3 access path without weakening the mailbox security model.

**Tasks:**

- Audit `smtp-pop3/src/lib.rs`; remove placeholder acceptance of arbitrary passwords and zero-message behavior.
- Implement a real bounded TCP/TLS listener, secure authentication, transaction/update state, mailbox snapshot and durable delete/quit semantics.
- Implement USER/PASS only inside the selected TLS policy; add SASL/OAuth only if required and fully reviewed.
- Implement STAT, LIST, RETR, DELE, NOOP, RSET, UIDL, TOP if supported, multiline dot-stuffing, message sizes, update state and error replies.
- Persist delete marks and commit deletion on successful QUIT according to POP3 semantics. Ensure concurrent sessions cannot corrupt mailbox state.
- Apply mailbox permissions, tenant limits, audit and rate controls.
- Do not advertise commands the service does not support. If POP3 is not a product requirement, explicitly document it as deferred rather than ship a fake server.

**Acceptance:** A real POP3 client can authenticate securely, list/retrieve/delete messages, and observe correct transaction/update behavior after reconnect.

### M11 — Control-plane API completeness and public contracts

**Objective:** Evolve the Go API into a reliable tenant management and developer API for the backend.

**Tasks:**

- Preserve the `/v1` API convention and update `docs/api/openapi.yaml` for every public contract change.
- Review all handlers under `smtp-api/internal/controlplane/` and ensure each production operation uses durable PostgreSQL repositories rather than process-local `Store` state.
- Implement user/tenant lifecycle, membership and role management, API-key creation/list/revoke/rotation, domain verification, DKIM setup, mailbox administration, send API, status/history, per-recipient activity, event subscriptions and usage endpoints.
- Provide message body/content retrieval only through authenticated mailbox permissions and explicit content scopes. Do not leak raw mail through generic activity endpoints.
- Add stable cursor pagination, filtering, sorting, request validation, consistent error shapes, idempotency for externally visible writes, and API version compatibility tests.
- Ensure OIDC issuer/audience/expiry/subject checks and tenant selection are robust. Add browser-session/CSRF controls only if a browser auth flow is part of the chosen product design.
- Use a persistent/distributed rate limiter for API keys, users, tenants and IP sources. The current in-memory `Store.Allow` behavior is not sufficient for multi-instance production.
- Move usage and quotas to durable, concurrency-safe metering. Define usage units and time windows; ensure idempotent accounting for retries.
- Add tenant-scoped audit events for authentication, key/domain/mailbox changes, sends, policy changes and administrator actions.
- Add API contract tests, authorization matrix tests, OpenAPI validation and SDK compatibility checks.

**Acceptance:** All documented control-plane operations are durable, tenant-safe and rate limited. API documentation matches actual handlers and response fields.

### M12 — Tenant plans, quotas, billing and abuse controls

**Objective:** Support safe multi-tenant service usage and commercial plan boundaries.

**Tasks:**

- Define tenant plans/entitlements for message volume, recipients per message, mailbox/attachment storage, API requests, domains, sending rates and retention.
- Implement quota reservation/consumption/release accounting around enqueue and delivery. Prevent race conditions and double counting on duplicate requests/retries.
- Add per-tenant, per-domain, per-user, per-IP and per-destination rate policies with safe defaults and administrative overrides.
- Implement sender-domain authorization, suppression lists, bounce/complaint controls, new-tenant warm-up, reputation-aware limits and emergency kill switches.
- Build a durable metering ledger and reconciliation jobs. Keep billing provider integration behind a provider interface and do not make payment-provider availability block delivery of already accepted mail.
- Add tenant suspension, domain disablement, key revocation and incident response workflows.
- Define trial/free-tier abuse controls and safe handling for compromised accounts.

**Acceptance:** A tenant cannot exceed its limits through concurrent requests, retry storms, API-key rotation, aliasing or mailbox fan-out. Usage can be reconciled from durable facts.

### M13 — Webhooks, event outbox and notifications

**Objective:** Deliver trustworthy platform events to tenants and internal consumers.

**Tasks:**

- Define versioned events for message accepted, remote accepted, delivered/bounced where known, deferred, suppressed, domain verified, mailbox changed, quota reached and security events.
- Use a transactional outbox for all events tied to database mutations. Do not rely on in-memory publish calls.
- Implement webhook subscription CRUD, event selection, endpoint validation, signed payloads, secret rotation and replay protection.
- Implement asynchronous delivery with exponential backoff, bounded concurrency, idempotency keys, delivery attempt history, dead-letter state and manual replay.
- Protect against SSRF when tenants register webhook targets: block private, loopback, link-local, metadata and reserved destination ranges; revalidate DNS at connect time; constrain redirects; support HTTPS and safe TLS verification.
- Ensure payloads contain only tenant-authorized metadata and do not expose message bodies or secrets by default.
- Document at-least-once webhook semantics, retry windows, signatures and replay behavior.

**Acceptance:** Events survive process restarts, duplicate delivery is safe, secrets are protected, and webhook URLs cannot be used to reach internal infrastructure.

### M14 — Durable object storage, attachments and malware scanning

**Objective:** Replace in-memory attachment storage with a secure production content pipeline.

**Tasks:**

- Introduce a durable object-storage interface and implement a selected S3-compatible backend with bounded streaming upload/download, multipart uploads, abort cleanup and retry-safe completion.
- Persist tenant-scoped attachment metadata and object references in PostgreSQL. Keep content hashes and deduplication scoped safely; never permit cross-tenant content disclosure through deduplication.
- Integrate the existing encryption/compression primitives only after key lifecycle and rotation are defined. Use envelope encryption with KMS-managed keys, versioned key IDs, authenticated encryption and re-encryption/rotation jobs.
- Replace the test scanner interface with an adapter to a real malware scanning service or isolated scanner process. Fail closed or quarantine on scanner outage according to a documented policy.
- Validate filenames, content type and content signatures; protect against path traversal, archive bombs, decompression bombs, active content and malformed file formats.
- Add safe content-disposition, range requests, download audit, retention and deletion propagation.
- Add quotas, orphan cleanup and object-store integrity reconciliation.

**Acceptance:** Attachments survive service restart and can be securely uploaded, scanned, retrieved, rotated, retained and deleted under tenant authorization.

### M15 — Search, threading, rules and mailbox synchronization

**Objective:** Make mailbox productivity capabilities durable and integrated.

**Tasks:**

- Connect `smtp-core/src/search.rs` and existing PostgreSQL search schema to MIME projections and real mailbox mutations. Build idempotent indexing workers, backfill/rebuild, index-version tracking and tenant-scoped query authorization.
- Implement a robust query grammar, bounded complexity, field filters, attachment search, date ranges, pagination and rank behavior.
- Persist conversation/thread relationships using RFC message identifiers (`Message-ID`, `In-Reply-To`, `References`) with malformed-header defense and deterministic merge/rebuild rules.
- Wire mailbox rules to inbound processing with explicit ordering, bounded execution, loop prevention, safe matching and audit metadata.
- Persist synchronization/change journals and expose a cursor API with replay, invalidation, tombstones and conflict metadata.
- Wire offline mutation primitives to a durable client sync contract; define idempotency and conflict resolution for concurrent actions.
- Add index lag, sync lag and rebuild monitoring.

**Acceptance:** Search, threads, rules and synchronization reflect persisted mailbox changes and remain correct after retry, restart, backfill and concurrent modification.

### M16 — Retention, compliance, audit and data governance

**Objective:** Make hosted message storage operable under privacy and compliance requirements.

**Tasks:**

- Define tenant-level retention policies for messages, attachments, queue payloads, delivery events, audit records, logs, backups and webhook payloads.
- Implement deletion workflows with soft-delete, purge scheduling, tombstones, object-store deletion and auditable completion. Respect legal hold where configured.
- Implement mailbox export/data portability and controlled account deletion.
- Add encryption-at-rest configuration and key rotation validation. Ensure backups are encrypted and restorable into isolated environments.
- Harden audit logs against tampering and unauthorized deletion. Separate security audit retention from customer content retention.
- Review data residency, cross-region processing, access review, incident evidence and privacy requirements before enabling new regions.
- Provide operational runbooks for legal requests and security incidents; do not automate legal decisions.

**Acceptance:** Retention, export, deletion, audit and recovery policies are enforced in code and validated with integration tests.

### M17 — Observability, SRE, performance and deployment hardening

**Objective:** Operate the platform with measurable reliability and capacity controls.

**Tasks:**

- Extend existing Go metrics and Rust tracing into end-to-end OpenTelemetry-compatible metrics, traces and structured logs.
- Instrument SMTP command latency, auth rejection, TLS failures, queue age/depth, attempt outcomes, remote SMTP response classes, delivery delay, bounce/complaint rates, mailbox growth, search lag, API errors and webhook retries.
- Use bounded-cardinality labels and no PII/content labels. Add trace/context propagation between Go, Rust and async workers.
- Define availability, latency, queue-drain, delivery and recovery SLOs with actionable alerts.
- Add resource limits, backpressure and capacity testing for connection count, message bytes, queue concurrency, database pools, DNS, object storage and indexing.
- Add database partitioning/retention strategies only after measured growth requires them. Validate indexes with query plans and representative data.
- Add production container images, non-root execution, read-only filesystems where feasible, SBOM, image/dependency scanning, provenance and signature verification.
- Add deployment promotion, readiness/draining, safe schema rollout, rollback/forward-fix strategy, secrets injection and incident runbooks.
- Keep health checks truthful: liveness should not depend on every external service; readiness should represent the dependencies required to accept traffic.

**Acceptance:** Load/failure tests demonstrate bounded behavior, observability answers the key operational questions, and deployment/rollback procedures are reproducible.

### M18 — High availability, global routing and disaster recovery

**Objective:** Evolve the existing topology model into real multi-zone and multi-region infrastructure.

**Tasks:**

- Define regional service ownership and data residency boundaries. Distinguish stateless API/SMTP edge services from regionally authoritative queue, mailbox, key and tenant data.
- Implement health-aware ingress routing, tenant home-region mapping, DNS/geo routing and safe regional failover.
- Define PostgreSQL HA, replication lag thresholds, failover and split-brain prevention. Do not allow two active regions to deliver duplicate queue work without a fencing/ownership protocol.
- Design object-storage replication, encryption-key availability, regional secrets and certificate management.
- Define queue failover, idempotent replay, mailbox consistency and outbox continuity across regions.
- Implement backup/restore drills, RPO/RTO targets, immutable backups and periodic recovery exercises.
- Extend `regional_topology.rs` only as part of real orchestration and service-health consumers; a valid topology data structure is not geo-routing.
- Add region-by-region rollout, traffic shift and emergency failback runbooks.

**Acceptance:** Documented and tested multi-zone/region failover meets measured RPO/RTO targets without violating tenant isolation or queue/message uniqueness semantics.

### M19 — AI, analytics and advanced reputation (optional, after core stability)

**Objective:** Add intelligent platform capabilities without allowing model output to compromise mail security or user control.

**Tasks:**

- First integrate deterministic rules and trusted signals for abuse, phishing, spam and anomaly detection.
- Treat the current `smtp-core/src/ai_*` modules as domain/reference primitives, not a live model service.
- Create provider/model abstractions only where more than one concrete consumer exists. Support self-hosted models or external APIs through explicit data processing policy, tenant consent, retention controls and redaction.
- Implement asynchronous classification with versioned model metadata, confidence, explainable reason codes, human review and safe fallbacks.
- Smart replies, summaries, translations and compose assistance must remain user-reviewed drafts. AI must never send, delete, forward, change security policy or release quarantined mail autonomously without a separate explicit authorization workflow.
- Add feature opt-in, data minimization, retention control, cost limits and per-tenant quotas.
- Build analytics from durable event/read models with privacy-safe aggregation, not by scanning logs or using in-memory counters.

**Acceptance:** AI and analytics are optional, observable, consent-aware, bounded in cost, and do not alter critical mail/security behavior without explicit policy.

### M20 — SDK, CLI and developer experience

**Objective:** Provide stable external integration interfaces after the API contracts are reliable.

**Tasks:**

- Generate or maintain official SDKs from the OpenAPI contract with typed requests/responses, pagination, retries, idempotency and error handling.
- Implement `smtp-cli/` with login/config validation, domain diagnostics, API-key management, send/test workflows, queue/activity inspection and safe output redaction.
- Add SDK integration tests against a local API fixture and compatibility tests for versioned API changes.
- Publish concise quickstarts, authentication guides, SMTP integration examples, webhook verification examples and migration notes.
- Avoid hand-maintaining duplicated client schemas when code generation can reliably consume the OpenAPI contract.

**Acceptance:** An external developer can authenticate, verify a domain, submit a message, inspect per-recipient status and verify webhooks using documented SDK/CLI flows.

---

## 9. Special implementation cautions

### 9.1 SMTP delivery semantics

The product must explicitly model at least these distinctions:

- `accepted_by_api`: API validated and authorized the request.
- `accepted_by_ingress`: SMTP server accepted the transaction according to its configured local policy.
- `persisted`: canonical message and required metadata are durable.
- `queued`: eligible for delivery processing.
- `deferred`: temporary failure; retry is scheduled.
- `accepted_by_remote`: the next-hop SMTP server returned successful final DATA acceptance.
- `bounced`: a permanent failure or a downstream DSN indicates non-delivery.
- `suppressed`: policy prevented send due to suppression/abuse/tenant state.
- `delivered_to_mailbox`: only use when the platform itself controls the destination mailbox and has durably stored the message.

Do not label `accepted_by_remote` as `delivered_to_inbox` or `delivered_to_mailbox` for arbitrary external destinations.

### 9.2 Database migrations

- Number new migrations after the highest existing version in `smtp-storage/postgres/`.
- Keep old migration files immutable.
- Add checks, foreign keys and tenant-aware indexes as part of the migration that introduces the feature.
- Provide a safe backfill plan and dual-read/dual-write or compatibility path for state/schema transitions where needed.
- Validate both a fresh schema and an upgrade from the previous supported schema.
- Do not drop or rewrite customer data as an incidental part of a refactor.

### 9.3 RFC and protocol claims

- Identify the RFC and exact feature profile before coding protocol behavior.
- Write tests against RFC examples and independent interoperability fixtures.
- Do not claim RFC-compliant support based only on parser acceptance or unit tests of a happy path.
- Keep unsupported extension behavior explicit and unadvertised.
- For crypto/authentication, use vetted algorithms and test vectors rather than custom approximations.

### 9.4 External infrastructure

- Do not enable public SMTP listeners, external delivery, object storage, Keycloak production, cloud KMS, Kubernetes, or multi-region routing by default just because code exists.
- Keep local/dev configuration isolated from production settings.
- Make every external dependency optional in tests through deterministic fakes, but validate the actual provider adapter in a dedicated integration environment before production rollout.

---

## 10. Required final report after each implementation run

After implementing the selected milestone, report:

1. **Milestone:** ID and title; whether fully complete, partially complete, or blocked.
2. **Repository findings:** existing code reused, important defects found, and assumptions clarified.
3. **Files changed:** paths grouped by service, persistence, tests, configuration and documentation.
4. **Database changes:** migration numbers, schema changes, indexes, backfill/rollback implications.
5. **Behavior delivered:** concrete end-to-end behavior now available and any limitations.
6. **Security:** tenant isolation, authentication, rate limiting, input bounds and secrets handling addressed.
7. **Validation:** exact commands executed and actual pass/fail results. Explicitly list skipped tests and why.
8. **Operational impact:** new configuration variables, worker/listener behavior, metrics, alerts and runbooks.
9. **Remaining work:** specific gaps and the next milestone ID in dependency order.

Never report a feature as complete just because files compile or interfaces exist. Never claim performance, scalability, RFC compliance, high availability, inbox delivery, or production readiness without corresponding evidence.

---

## 11. First implementation priority

Unless the repository has materially changed since inspection, start with **M00**, then immediately prioritize the security/correctness defects in **M01**. In particular, fix the SMTP AUTH TLS-policy bypass and implement/test ingress STARTTLS before exposing authenticated SMTP submission beyond a strictly isolated development listener.

Next, implement M02–M06 in order to establish a correct message lifecycle and reliable inbound/outbound delivery. Do not prioritize AI, advanced analytics, dashboard polish, Kubernetes, or global multi-region orchestration ahead of secure transport, durable delivery semantics, mailbox isolation, and protocol correctness.

**Final instruction:** Build this as one coherent email platform. Reuse existing working code, connect isolated components, replace placeholders with real implementations, and prove every feature with tests and operational evidence. Keep the system secure-by-default and honest about its delivery guarantees at every stage.
