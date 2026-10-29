# Phase implementation status and deferred work

This register is the authoritative cross-phase record of what is implemented,
what remains, why it is not implemented yet, and when the remaining work should
be applied. “Deferred” does not mean forgotten: each item has an owner phase or
an explicit prerequisite.

For the connected handoff analysis and the completion boundary through Phase
10, see the [Phase 01–10 integration review](phase-integration.md).

The repository is intentionally a development and validation foundation. A
feature must not be described as production-ready merely because its
architecture, interface, prototype, or documentation exists.

## Phase 11 — Email protocol engine

**Status:** Foundation research and bounded envelope validation implemented;
full protocol conformance remains in progress.

**Implemented:** The [Phase 11 protocol-engine document](standards/phase-11-email-protocol-engine.md)
records the RFC-backed message layers, lifecycle, acceptance boundary, and
deferred milestones. Queue validation now preserves the distinction between
SMTP envelope paths and RFC 5322 message content, including support for the
null reverse-path used by generated mail.

**Remaining implementation:** Complete SMTP state/reply conformance fixtures,
bounded MIME parsing, submission authentication, SMTPUTF8/IDNA, DKIM/DSN, and
any selected mailbox protocol.

**Reason and application point:** These features have cross-service storage,
security, and interoperability consequences. Apply them as dedicated
milestones after their product profile and test fixtures are approved; this
phase establishes the contracts they must preserve.

## Phase 12 — SMTP core engine

**Status:** Bounded RFC 5321 core profile implemented and tested.

**Implemented:** The SMTP server now advertises its configured `SIZE` limit,
uses the configured server name, accepts the null reverse-path, enforces the
core transaction sequence and DATA framing, and keeps unsupported extensions
unadvertised. The outbound SMTP client remains bounded and disabled for real
external delivery by default.

**Remaining implementation:** STARTTLS, submission authentication, DNS and
destination policy, external-delivery enablement, PIPELINING, DSN,
SMTPUTF8/REQUIRETLS, and interoperability testing are separate production
hardening milestones.

## Phase 13 — SMTP session state machine

**Status:** Bounded submission state machine and injectable AUTH PLAIN flow
implemented and tested.

**Implemented:** The session owns explicit connection, greeting,
authentication, envelope, recipient, DATA, and closed states. `RSET` preserves
authentication, AUTH continuation is bounded, credentials are delegated to an
injectable verifier, and authentication is advertised only when configured.

**Remaining implementation:** TLS-backed authentication, durable credential
verification, STARTTLS state reset, production authorization policy, and
external interoperability testing.

## Phase 14 — SMTP security

**Status:** Secure transport foundation implemented; STARTTLS upgrade remains
pending.

**Implemented:** Certificate-backed Rustls server configuration, optional
implicit TLS, explicit TLS failure handling, and plaintext AUTH rejection by
default.

**Remaining implementation:** In-place STARTTLS upgrade and transaction reset,
TLS-backed end-to-end AUTH tests, certificate rotation, client certificates,
and production secret-manager integration.

## Phase 15 — SMTP extensions (ESMTP)

**Status:** Capability-gated `SIZE` and optional `8BITMIME` foundation
implemented; the complete extension profile remains in progress.

**Implemented:** EHLO `SIZE`, bounded `MAIL FROM SIZE=`, opt-in `8BITMIME`,
`BODY=7BIT|8BITMIME` parsing, and rejection of unknown ESMTP parameters.

**Remaining implementation:** Safe `PIPELINING`, `CHUNKING`/`BDAT`,
SMTPUTF8/Unicode storage and delivery, and DSN parameter and report handling.
Unsupported extensions remain unadvertised.

## Phases 16–20 — Queue, delivery, inbound, MIME, and message parsing

The phase-specific contracts and current boundaries are documented under
[`standards/`](standards/). Phase 16 has received a schema correction for the
SMTP null reverse-path. The existing queue, delivery, and inbound components
are foundations, not claims of Brevo/Google-scale production readiness.
Phases 17–20 must be implemented in dependency order: queue outcomes,
delivery/inbound storage, bounded message parsing, then MIME projection.

## Phases 21–25 — Mailbox protocols and storage

Research and storage contracts are documented under [`standards/`](standards/).
Phase 21/25 now has a durable folder/message schema and shared bounded mailbox
model. IMAP and POP3 listeners and command implementations remain pending and
must be built on this storage contract rather than independent in-memory state.

## Phases 26–30 — Search, conversations, attachments, and synchronization

Research contracts are documented under [`standards/`](standards/). The shared
core now contains bounded reference implementations for search projection,
reference-based conversation grouping, SHA-256 attachment deduplication with
injected scanning, monotonic synchronization cursors, and an offline mutation
queue.

**Status:** Foundation implemented; durable integration is pending.

**Remaining implementation:** Persist search projections, thread relationships,
attachment metadata/content, and change journals in PostgreSQL/object storage;
connect them to MIME parsing and IMAP mailbox mutations; add tenant-aware
authorization, quotas, replay/rebuild jobs, conflict metadata, client
pagination, cursor invalidation, and operational observability. The in-process
implementations are testable domain references and are not a production search
cluster, antivirus service, object store, or offline client database.

**Reason and application point:** These components depend on the complete MIME
and message-parser contracts from Phases 19–20 and the durable mailbox
mutation APIs from Phases 21–25. Apply the storage adapters and interoperability
tests before enabling hosted-mailbox search or synchronization for customers.

## Phases 31–35 — DNS and email authentication

The standards baseline is documented under [`standards/`](standards/). The
shared core now contains typed DNS cache records, bounded SPF qualifier
evaluation, injectable DKIM canonicalization/signing/verification boundaries,
and strict-domain DMARC alignment evaluation.

**Status:** Authentication foundation implemented; live DNS, key management,
cryptographic adapters, reporting, and enforcement integration are pending.

**Remaining implementation:** Add TTL- and DNSSEC-aware resolver adapters,
complete SPF include/redirect and lookup-limit handling, integrate protected
RSA/Ed25519 key services, publish and rotate DKIM selectors, implement full
header/signature parsing, support DMARC organizational-domain alignment and
policy discovery, and persist aggregate/forensic reports. These boundaries
must be wired into inbound/outbound decisions with explicit tenant policy and
observability before production use.

## Phases 36–40 — ARC, BIMI, unified authentication, DSN, and compliance

Phase contracts are documented under [`standards/`](standards/). The core now
has an ARC chain structure validator and injected verifier boundary, basic BIMI
record/HTTPS URL validation, a unified authentication result that keeps ARC and
BIMI from automatically overriding DMARC policy, recipient-scoped DSN and ARF
feedback primitives, and a deterministic compliance fixture runner.

**Status:** Foundation implemented; protocol-complete cryptography, durable
delivery tracking, deployment integrations, and interoperability evidence are
pending.

**Remaining implementation:** Implement ARC cryptography and header semantics;
BIMI retrieval, SVG/certificate validation, and provider eligibility; connect
authentication outcomes to inbound policy; persist delivery reports and emit
standards-conformant DSNs/ARFs; add RFC-indexed negative/fuzz/load/recovery
coverage and controlled provider interoperability. No provider certification
or production-readiness claim is made by unit-test results alone.

## Phase 00 — Product vision and project strategy

**Status:** Documented and used as the scope baseline.

**Implemented:** The repository scope is a multi-tenant transactional email
delivery foundation with explicit extension paths for inbound mail, hosted
mailboxes, and a managed email cloud. Licensing, contribution, security
reporting, and product limitations are documented in the root project
documentation and security policy.

**Remaining implementation:** No customer-facing product code is expected from
this phase. The missing strategy work is to approve the initial commercial
product profile, launch regions, support/SLO targets, retention policy, and
abuse/compliance operating model.

**Reason and application point:** These decisions depend on customer discovery,
legal review, and operational capacity rather than repository code. Apply them
before Phase 10 production readiness and before enabling public customer
traffic. They are prerequisites for final quotas, retention, support, and
regional deployment decisions.

## Phase 01 — System architecture

**Status:** Architecture documented and aligned with the current prototype.

**Implemented:** Service boundaries, message lifecycle, PostgreSQL-first
durability, queue lease semantics, outbound destination safety, inbound opt-in
scope, reliability assumptions, security boundaries, and extension paths are
documented under [`architecture/phase-01/`](architecture/phase-01/).

**Remaining implementation:**

- Replace the in-memory Go control-plane store with PostgreSQL repositories.
- Choose the production deployment/runtime and service topology.
- Define the production event publication adapter and delivery read model.
- Complete sender authorization, bounce/suppression, and abuse-control
  boundaries at the send API.
- Decide whether hosted mailboxes and inbound reception are launch features.

**Reason and application point:** Phase 01 intentionally established boundaries
without implementing application services or locking in unvalidated cloud
choices. Apply repository and deployment work in Phases 06, 08, 09, and 10;
apply hosted-mailbox extensions only after a product decision and dedicated
protocol/storage milestones.

## Phase 02 — Standards and RFC research

**Status:** Research baseline complete; protocol conformance is not complete.

**Implemented:** RFC and IANA research, normative requirement traceability,
protocol feature mapping, standards governance, and conformance-test planning
are documented under [`standards/`](standards/).

**Remaining implementation:**

- Implement and test SMTP submission authentication and the selected profile.
- Implement DKIM signing and key lifecycle.
- Implement standards-conformant bounce/DSN and delivery-status handling.
- Add SMTPUTF8/IDNA behavior if required by the launch profile.
- Implement MTA-STS, TLS reporting, DANE, REQUIRETLS, ARC, or mailbox
  protocols only when selected by the product profile.
- Build interoperability and fuzz/conformance fixtures against approved peers.

**Reason and application point:** The phase records requirements but does not
pretend that reading an RFC is implementation. Apply baseline SMTP/MIME and
authentication behavior in Phase 08; apply DKIM, bounce, and reputation
features in the delivery milestones; apply optional interoperability features
only after the unresolved standards questions and deployment profile are
approved.

## Phase 03 — Development environment

**Status:** Local development workflow implemented; production environment is
not provided.

**Implemented:** Toolchain pinning, Make targets, local Compose PostgreSQL,
bootstrap and runner scripts, environment templates, formatting/lint/test
commands, contributor guidance, and development safety checks.

**Remaining implementation:**

- Add supported production deployment manifests or a selected deployment
  target.
- Validate macOS/Windows contributor workflows beyond documented guidance.
- Add reproducible production-like integration environments.
- Resolve the local default-port limitation through environment-specific
  configuration rather than changing secure defaults.

**Reason and application point:** Production infrastructure was explicitly
outside the development-environment phase, and cloud/provider selection is
still open. Apply deployment manifests and production-like environments in
Phase 10 after the runtime, database, secret manager, and observability
decisions are approved.

## Phase 04 — Core infrastructure

**Status:** Shared Rust infrastructure implemented; production adapters remain
deliberately incomplete.

**Implemented:** Typed configuration and validation, secret-provider
interfaces, redacted secret diagnostics, context identifiers, shared errors,
versioned event envelopes, logging initialization, shutdown signaling, and the
shared PostgreSQL connection foundation.

**Remaining implementation:**

- Add a production secret-manager provider.
- Add a durable event/outbox publisher and consumer adapter.
- Add cross-service trace/context propagation at actual API and event
  boundaries.
- Add production-grade configuration sources and secret rotation hooks.
- Add durable audit/event delivery guarantees once consuming services exist.

**Reason and application point:** Phase 04 created contracts without fake
brokers, unsafe local secret integrations, or abstractions with no consumer.
Apply the secret provider in Phase 07/10 when a production manager is selected;
apply the event/outbox adapter when Phase 08 services publish real events; add
propagation when service-to-service boundaries are implemented.

## Phase 05 — DevOps and CI/CD

**Status:** CI, security scanning, container validation, and release packaging
implemented; production release operations are not complete.

**Implemented:** Rust, Go, dashboard, migration, formatting, test, build,
dependency, secret, workflow-guardrail, container-configuration, and trusted
tag release checks. Known RustSec findings remain visible and unsuppressed.

**Remaining implementation:**

- Migrate workflow action references to reviewed commit SHAs.
- Add container image builds, image scanning, SBOM, and provenance.
- Configure protected branches, required checks, release approvals, and
  artifact retention.
- Add deployment promotion, rollback, and environment-specific secret
  handling.
- Remediate or isolate the unresolved `idna`, `rsa`, and unmaintained
  `trust-dns` dependency findings.

**Reason and application point:** Remote repository settings, registries,
cloud credentials, and deployment targets are unavailable or intentionally
unselected. Apply repository governance and artifact hardening before the first
production release; apply deployment promotion in Phase 10; resolve dependency
findings before enabling public delivery.

## Phase 06 — Database and storage architecture

**Status:** PostgreSQL foundation, migrations, queue persistence, backup, and
restore validation complete.

**Implemented:** Forward-only migrations, PostgreSQL pool configuration,
statement timeouts, durable queue state, control-plane schema, inbound
metadata/content persistence, migration validation, backup creation, isolated
restore testing, and storage policies.

**Remaining implementation:**

- Replace in-memory control-plane state with PostgreSQL repositories.
- Add durable quota, usage, delivery-event, bounce, suppression, and audit
  read models.
- Define and implement production retention, deletion, and legal-hold rules.
- Select object storage when message size, retention, or scaling requires it.
- Select Redis only for an approved cache/coordination consumer.
- Add encrypted production backups, restore automation, and access review.

**Reason and application point:** PostgreSQL is sufficient for the current
bounded prototype; Redis and object storage have no approved current consumer.
Apply repository/read-model work in Phase 08, retention and backup operations in
Phase 10, and object storage only after measured payload/scale requirements
justify the added consistency boundary.

## Phase 07 — Security foundation

**Status:** Local security primitives and threat-model documentation
implemented; production identity and key-management integrations remain
deferred.

**Implemented:** Typed human/service identities, default-deny authorization,
resource tenant checks, scoped API-key verification, constant-time comparison,
expiry/revocation checks, CSPRNG identifiers and secrets, development-mode
production guard, audit safety guidance, threat modeling, incident guidance,
and security regression tests.

**Remaining implementation:**

- Integrate an approved production OIDC issuer with issuer, audience,
  signature, expiry, key-selection, and revocation validation.
- Persist API-key verifiers and audit events through PostgreSQL repositories.
- Add production secret-manager, KMS/HSM, certificate, and key-rotation
  integrations.
- Establish workload identity or mTLS for internal service calls.
- Add durable audit export, alerting, access review, and incident drills.
- Resolve the dependency advisories documented by the security workflow.

**Reason and application point:** The identity provider, production secret
manager, KMS/HSM, certificate authority, and deployment runtime are not yet
selected. Applying these controls now would create an unowned or misleading
production security implementation. Apply OIDC, durable credentials, and
service identity in Phase 08 when API/service boundaries are implemented;
apply KMS, certificates, audit operations, and incident drills in Phase 10
before public launch.

## Phase 08 — API and communication framework

**Status:** Core HTTP API conventions implemented; distributed communication
guarantees and production integrations remain deferred.

**Implemented:** Existing `/v1` Go routes now use bounded request IDs, stable
nested error responses, strict JSON decoding, content-type checks, and
validated list limits. Phase 07 authentication and tenant authorization remain
the security boundary. Handler tests cover request-ID behavior, strict input,
error serialization, and invalid pagination. OpenAPI documentation and the
API developer guide describe the actual surface.

**Remaining implementation:**

- Replace the in-memory control-plane store with PostgreSQL-backed API
  repositories.
- Add durable idempotency for externally visible mutations once the relevant
  database tables and operation semantics are approved.
- Add authenticated internal service clients or RPC only when a second service
  has a real request/response consumer.
- Add a durable event publisher/outbox and consumer retry/dead-letter behavior
  when event-producing workflows are implemented.
- Add distributed rate limiting and full trace propagation with the Phase 09
  deployment/observability infrastructure.
- Add automated OpenAPI drift validation when a repository-supported validator
  is selected.

**Reason and application point:** The current API has one Go HTTP consumer and
an intentionally in-memory development store. Adding gRPC, a generic client,
process-local idempotency, or a fake broker would create misleading guarantees.
Apply PostgreSQL-backed mutations and idempotency in the API/data-plane
milestone after Phase 06 repositories are wired; apply internal identity and
event delivery when multiple services communicate; apply distributed limits,
full tracing, and contract automation in Phases 09–10.

## Phase 09 — Monitoring and observability

**Status:** Local API observability foundation implemented; centralized
telemetry and production operations remain deferred.

**Implemented:** The Go API now has structured request-completion logs,
bounded aggregate HTTP metrics at `/metrics`, separate liveness and readiness
endpoints, metric-cardinality protections, health tests, observability tests,
Prometheus-compatible provisional alert rules, a dashboard definition, an
operations runbook, and an observability ADR. Existing Rust `tracing` logging
and distinct request/correlation/trace identifiers are preserved.

**Remaining implementation:**

- Add a selected OpenTelemetry exporter and W3C trace-context propagation
  when a collector and multiple service boundaries exist.
- Replace process-local counters with a Prometheus-compatible or equivalent
  distributed scrape/storage deployment.
- Integrate centralized log collection, retention, access control, and
  redaction review.
- Add dependency/database/worker/event metrics when those production consumers
  are implemented.
- Deploy and authenticate dashboards and alert notification routing.
- Replace provisional alert thresholds with Phase 10 workload/failure evidence.
- Measure telemetry overhead, shutdown/export behavior, and loss characteristics
  under representative load.

**Reason and application point:** The current repository has one Go API
consumer, no collector, no centralized telemetry backend, and no production
deployment target. Adding a full monitoring stack now would create
unowned infrastructure and unsupported reliability claims. Apply collector,
distributed metrics, and authenticated dashboards when the deployment target
is selected; add cross-service tracing and dependency instrumentation as
services are introduced; finalize SLOs and alert thresholds during Phase 10
validation.

## Completion rule

A phase may be marked complete for its documented scope while still having
deferred implementation. The phase is not production-ready until every item
listed under its remaining implementation has either been completed, replaced
by an accepted architecture decision, or explicitly accepted as a
time-bounded launch exception.

## Phase 10 — Platform foundation validation

**Status:** Validated with limitations; not production-ready.

**Implemented:** Repository-wide Rust and Go validation, Go race testing,
OpenAPI/dashboard checks, live API health and metrics smoke tests, isolated
PostgreSQL migrations, queue integration tests, backup inspection, isolated
restore validation, a controlled API performance smoke baseline, a
requirements-to-evidence matrix, and a prioritized risk register.

**Remaining implementation:**

- Remediate the failing RustSec findings before public production delivery.
- Replace in-memory control-plane state with durable PostgreSQL repositories.
- Configure production OIDC, secret management, certificates, and service
  identity.
- Deploy authenticated centralized metrics, logs, traces, dashboards, and
  alert routing.
- Run representative concurrent load, dependency-failure, and recovery tests.
- Execute Gitleaks and verify remote CI/branch-protection settings.

**Reason and application point:** The current repository supports safe local
and isolated validation, but lacks the production deployment, identity,
telemetry, and workload environment needed for a stronger readiness claim.
Apply the infrastructure and dependency remediations before public launch;
repeat performance and recovery validation after the production-like runtime
is selected. The evidence and exact limitations are maintained in
[`validation/`](validation/).

## Phase 41 — Mailbox platform architecture

**Status:** Architecture and storage boundaries documented; full service
integration remains pending.

**Implemented:** The architecture record defines tenant/domain/mailbox
ownership, alias and shared-mailbox semantics, mailbox lifecycle, quota
accounting, PostgreSQL metadata, object-storage migration target, authorization
boundaries, API target, and production acceptance evidence. The migration
enforces tenant-consistent mailbox/domain references.

**Remaining implementation:** Deployable object storage, production API and
repository integration, encryption and key lifecycle, capacity/partition
strategy backed by workload evidence, disaster recovery, and load/failover
validation.

**Reason and application point:** The current API store is process-local and
message bytes remain in PostgreSQL. Apply the architecture incrementally in
Phases 42–45; do not enable hosted mailbox traffic until storage, auth,
retention, operations, and recovery criteria are evidenced.

## Phase 42 — User mailboxes

**Status:** Database schema foundation implemented; lifecycle and API
operations are not yet wired.

**Implemented:** Mailbox type, optional tenant-member owner, quota fields,
lifecycle statuses, same-tenant aliases, and mailbox-scoped permissions are
added in migration `000008`. Rust storage primitives provide transactional
mailbox provisioning with default folders, lifecycle transitions, alias
creation, and permission grant/revocation.

**Remaining implementation:** Transactional create/provision/activate,
idempotent alias and permission APIs, audited lifecycle transitions,
authorization integration, inbound routing to mailbox IDs, retention-aware
deletion, and PostgreSQL-backed control-plane handlers.

**Reason and application point:** Existing control-plane mailbox endpoints
read in-memory maps. A production mailbox API must be built on durable
repositories and the Phase 07 authorization adapter rather than layering
write routes over ephemeral state.

## Phase 43 — Folder management

**Status:** Durable nested-folder primitives implemented; folder mutation
protocols and synchronization remain pending.

**Implemented:** Same-mailbox parent constraints, sibling/root uniqueness,
special-use metadata, stable UIDNEXT, idempotent default-folder creation, and
transactional append UID allocation are present.

**Remaining implementation:** Rename/move/delete semantics, folder metadata
and ACL APIs, IMAP LIST/STATUS behavior, durable sync journal, audit events,
and concurrent protocol integration tests.

**Reason and application point:** Folder metadata must remain consistent with
IMAP UID and synchronization rules. Complete these operations with the Phase
21–23 IMAP service and durable Phase 29 synchronization integration.

## Phase 44 — Message storage engine

**Status:** Transactional inline PostgreSQL prototype; scalable storage
backend remains pending.

**Implemented:** Message append reserves raw-byte quota and advances folder
UIDNEXT in the same PostgreSQL transaction; expunge releases the charged
bytes. A hash projection column is available for future integrity checks.

**Remaining implementation:** Object-store adapter, streaming, encryption,
compression codec/versioning, same-tenant content deduplication, idempotent
append, orphan cleanup, outbox integration, MIME attachment references,
backup/restore, and workload-based scale validation.

**Reason and application point:** Inline `BYTEA` is the current bounded
implementation, not a Google/Brevo-scale storage guarantee. Adopt object
storage only with atomic visibility and cleanup semantics and verified
recovery procedures.

## Phase 45 — Message metadata

**Status:** Schema projection foundation implemented; parsing and mutation
integration remain pending.

**Implemented:** The schema provides header identifiers, references, subject,
sent time, conversation ID, labels, and existing flags/internal timestamp.

**Remaining implementation:** Bounded RFC/MIME projection, read/unread and
priority APIs, conversation reconciliation, label semantics, durable
synchronization events, search-index rebuild, and authorization checks for
every projection.

**Reason and application point:** Metadata must be parsed from canonical
message bytes and treated as a rebuildable projection. Wire it after the MIME
and parser contracts are reliable; never use untrusted headers as internal
timestamps or authorization data.

## Phase 46 — Attachment storage

**Status:** Encrypted, compressed, tenant-scoped reference store implemented;
durable storage and download service remain pending.

**Implemented:** Bounded chunk uploads, strict metadata checks, required
scanner injection, tenant-scoped SHA-256 deduplication, XChaCha20-Poly1305
encryption with tenant-associated data, gzip compression, key-version
selection, integrity verification, bounded chunk downloads, safe
Content-Disposition formatting, and plain-text-only previews.

**Remaining implementation:** Durable object-store adapter, streaming without
whole-attachment buffering, persistent resumable uploads, quotas,
scanner/quarantine orchestration, signed download URLs, audit/retention, KMS
integration, and failure/load/recovery tests.

**Reason and application point:** The in-memory implementation demonstrates
the storage contract but loses state on restart and keeps keys in process
memory. It must not be used as hosted production storage. Connect it to
Phase 44's object metadata and Phase 09/10's production key and operations
providers before exposing attachment endpoints.

## Phase 47 — MIME parsing

**Status:** Bounded MIME parser implementation and multipart tests added;
complete conformance and inbound integration remain pending.

**Implemented:** A standards-oriented parser extracts encoded subject and
addresses, nested plain/HTML bodies, embedded images, attachments, message
references, and decode-warning counts under explicit input, nesting, part,
attachment, and decoded-size limits.

**Remaining implementation:** Parser-to-inbound/mailbox integration,
quarantine/error policy, scanner handoff, extensive malformed/fuzz/RFC
fixtures, CPU budgets and worker isolation, and safe HTML rendering.

**Reason and application point:** A parser projection must be attached to a
durable message transaction without modifying the canonical RFC message.
Enable indexing only after the asynchronous projection and quarantine
contracts are available.

## Phase 48 — Mail indexing

**Status:** PostgreSQL search projection and indexes implemented; automatic
projection population remains pending.

**Implemented:** Migration `000009` adds sender/recipient/body/attachment
projection fields, weighted generated `tsvector`, partial live-message GIN
and B-tree indexes, trigram indexes, and per-mailbox indexing progress.

**Remaining implementation:** Idempotent asynchronous index worker, outbox
replay, delete propagation, authorization-aware population, rebuild/repair
jobs, and query-plan/write-amplification evidence.

**Reason and application point:** Mail append does not yet invoke the MIME
parser and populate the new projection columns; schema alone does not make
messages searchable. Populate through a replayable outbox after parser and
storage transactions are connected.

## Phase 49 — Full-text search

**Status:** Bounded tenant/mailbox-scoped PostgreSQL search function
implemented; customer API and enterprise query semantics remain pending.

**Implemented:** PostgreSQL web-search syntax, quoted terms, boolean/exclusion
operators, weighted ranking, optional sender/subject trigram matching, bounded
pagination, and tenant/mailbox authorization predicates are exposed as a
shared-core async function.

**Remaining implementation:** Authenticated search route, stable cursor
pagination, fielded date/from/to filters, highlighting, multilingual
stemming, relevance evaluation, abuse controls, and latency/freshness SLO
evidence.

**Reason and application point:** The function queries parser-populated
projection fields and is not reachable through the current HTTP API. Wire it
only after index freshness, mailbox permission checks, and product query
semantics are complete.

## Phase 50 — Search optimization

**Status:** Index and query optimization foundation implemented; measured
production tuning remains pending.

**Implemented:** Indexes use targeted GIN, trigram, and B-tree strategies and
exclude soft-deleted messages where appropriate. Search limits page size and
query length; mailbox-scoped index state supports future reindex operations.

**Remaining implementation:** Projection/cache pipeline, invalidation,
reindex throttling, representative EXPLAIN/ANALYZE and load tests,
tenant-skew capacity evidence, and any distributed backend decision.

**Reason and application point:** There is no approved production corpus,
latency target, or deployment profile against which to justify distributed
search or cache design. Measure PostgreSQL behavior first and add complexity
only when workload evidence demonstrates the need.

## Phase 51 — Conversation threading

**Status:** MIME-derived conversation linkage is written by the PostgreSQL
append path; full RFC threading/reconciliation remains pending.

**Implemented:** The in-memory thread index validates/canonicalizes
Message-IDs, groups unresolved references provisionally, merges groups
connected by later references, and removes message membership. PostgreSQL
append parses MIME threading headers and stores a mailbox-scoped conversation
link alongside the message.

**Remaining implementation:** Durable late-ancestor reconciliation/rebuild,
RFC 5256 algorithm output, explicit duplicate-ID policy, and concurrent/replay
tests at production scale. No IMAP threading capability is implied.

## Phase 52 — Mail filters

**Status:** Bounded rules are persisted and evaluated during PostgreSQL
mailbox append; SMTP ingress/submission integration remains pending.

**Implemented:** The rules planner evaluates bounded structured conditions in
deterministic priority order. The PostgreSQL append transaction evaluates
enabled incoming/bidirectional rules and applies supported folder, label, and
flag actions, recording rule executions atomically.

**Remaining implementation:** SMTP ingress/submission adapters, authenticated
HTTP CRUD, Sieve interoperability if selected, audit/quotas, and external
delivery failure/retry testing. Rules currently execute only when callers use
the PostgreSQL append path.

## Phase 53 — Rule engine

**Status:** Bounded evaluator and transactional append-time action execution
implemented; asynchronous rule workers and delivery adapters remain pending.

**Implemented:** Typed boolean/text/size/attachment predicates; bounded
expression depth, rule count, action count, and projection fields; stable
priority/ID ordering; and stop-after-match behavior.

**Remaining implementation:** Durable asynchronous executor, tenant fairness,
external delivery adapters, retry/dead-letter operations, versioned stored-rule
decoding, and production abuse/load testing. The mailbox outbox supports
lease/ack/retry but no broker consumer is wired.

## Phase 54 — Labels and categories

**Status:** Permission-checked PostgreSQL label create/assign operations and
transactional mailbox change journaling implemented; public APIs remain pending.

**Implemented:** Migration `000010` adds labels, categories, tags, virtual
smart-label definitions, cross-mailbox-safe assignments, and migrates legacy
text labels. Smart labels are represented as query-backed metadata.

**Remaining implementation:** Label update/delete and bulk APIs, smart-query
validation/execution, HTTP/protocol mapping, and migration/load testing at
realistic data volume.

## Phase 55 — Tags and custom properties

**Status:** Bounded JSONB property storage and mailbox-owned tag labels exist;
property mutation APIs and public query contract remain pending.

**Implemented:** Tags use mailbox-owned label assignments; messages have a
bounded JSONB custom-property projection and index. These properties do not
rewrite canonical message bytes.

**Remaining implementation:** Property mutation/validation API, reserved-key
and privacy policy, selective query support, audit/export/deletion semantics,
and isolation/load tests.

## Phase 56 — Spam folder management

**Status:** Permission-checked quarantine and release repository operations
implemented; classifier integration and scheduled review/expiry remain pending.

**Implemented:** Migration `000010` stores mailbox-scoped quarantine state,
reason, original-folder reference, and review deadline; existing `\Junk`
special-use folders remain the user-facing mailbox primitive.

**Remaining implementation:** Classifier integration, public review/reporting
API, scheduled expiry worker, audit operations, and retention/legal review.
Moves and change-journal writes are transactional through the repository.

## Phase 57 — Trash management

**Status:** Permission-checked Trash move/restore/purge operations implemented;
scheduled retention workers and protocol integration remain pending.

**Implemented:** Migration `000010` records the original folder/UID and
optional purge deadline independently from IMAP `\Deleted`/expunge state.

**Remaining implementation:** Scheduled cleanup, configurable retention
operations, IMAP/API wiring, and recovery/load tests. Restore allocates a new
UID, change-journal updates are transactional, and active legal holds block
purge; protocol expunge behavior is unchanged.

## Phase 58 — Archive system

**Status:** Permission-checked archive policy metadata and legal-hold-aware
Trash purging implemented; cold storage/retrieval is not.

**Implemented:** Migration `000010` stores archive tier, retention deadline,
and legal-hold metadata. Message bodies remain inline PostgreSQL `BYTEA`.

**Remaining implementation:** Object storage and tier/restore workers,
immutable retention policy, retrieval workflow, encryption/key lifecycle,
audit, recovery, and measured retrieval SLAs. Message bodies remain inline in
PostgreSQL and setting policy does not move message data.

## Phase 59 — Draft management

**Status:** Permission-checked immutable draft revision saves with optimistic
version checks, transactional quota charging, and mailbox change journaling
implemented; public autosave/sync APIs remain pending.

**Implemented:** Migration `000010` adds per-draft revision metadata and
immutable full-message snapshots. Database triggers charge and release the
snapshot bytes against mailbox quota atomically.

**Remaining implementation:** Public autosave/version/restore endpoints,
cross-device protocol sync, revision-retention worker, and production
concurrency/quota/load tests.

## Phase 60 — Sent mail management

**Status:** Permission-checked sent-submission and idempotent recipient-event
repository operations implemented; outbound queue/DSN integration remains
pending.

**Implemented:** Migration `000010` records idempotent submission linkage and
per-recipient state projections. It distinguishes accepted/queued state from
delivery state.

**Remaining implementation:** SMTP/API submission and sent-copy transaction,
queue/DSN event consumer, recipient-data retention, external delivery
synchronization, and end-to-end delivery tests.

## Phase 51–60 integration boundary

The PostgreSQL repository APIs and migration are implemented and covered by
the opt-in PostgreSQL integration test. They are not yet exposed through
authenticated Go HTTP endpoints, and inbound/outbound SMTP services do not yet
invoke these workflows. The outbox is a durable lease/ack/retry primitive,
not a running broker worker. No claim of production readiness, service SLO,
load capacity, disaster-recovery evidence, or compliance certification is
made.

## Phase 61 — Favorites and stars

**Status:** IMAP `\Flagged` storage and permission-checked mutation implemented;
public API and presentation remain pending.

**Implemented:** Message flags already support `\Flagged`; authorized writes
now enforce mailbox read-write permissions and flow through the durable
message-change journal. This is the interoperable star state, not a separate
per-user favorite or pin model.

**Remaining implementation:** IMAP flag command integration, authenticated
HTTP routes, pinned views, and per-user favorites if the product requires
different semantics.

## Phase 62 — Read-status synchronization

**Status:** Message flag sync plus auxiliary label, draft, and custom-property
journaling implemented; protocol cursor recovery remains pending.

**Implemented:** Durable change reads are mailbox-authorized and bounded.
Migration `000011` journals label assignments, draft revision inserts, and
custom-property updates into mailbox sync and the outbox, in addition to
message flags/moves/deletes already journaled.

**Remaining implementation:** Cursor expiry/full-resync, retention and
compaction, IMAP mod-sequences/QRESYNC, event consumers, and client adapters.

## Phase 63 — Mailbox quotas

**Status:** Transactional append/draft quota enforcement, authorized usage
reads, and quota updates implemented; notifications remain pending.

**Implemented:** The repository reports usage and utilization, returns
approaching-limit (80%) and near-limit (95%) warnings, enforces writes
transactionally, and rejects quota decreases below current usage.

**Remaining implementation:** Warning delivery/history, accounting for
object-store and attachment tiers, authenticated APIs, and concurrent load
evidence.

## Phase 64 — Retention policies

**Status:** Message-level retention metadata, legal holds, and guarded Trash
purge exist; scheduled policy execution remains pending.

**Implemented:** Legal hold is checked by Trash cleanup. Per-message policy
metadata can be stored through the workflow repository.

**Remaining implementation:** Approved mailbox/tenant retention policy,
audited changes, scheduled expiry/deletion workers, jurisdictional review, and
recovery/deletion evidence. No automatic mail expiration is enabled.

## Phase 65 — Mail export and import

**Status:** Raw RFC message bytes are retained; import/export is not implemented.

**Remaining implementation:** Bounded streaming EML/MBOX jobs, explicit MBOX
variant behavior, validation and idempotent import batches, export manifests,
and a separately selected PST conversion tool. No PST implementation is
claimed.

## Phase 66 — Backup and recovery

**Status:** Local PostgreSQL backup/isolated restore procedure documented;
production backup service and PITR are not configured.

**Remaining implementation:** Protected backup target, encryption and key
lifecycle, WAL archive/PITR, object-store consistency, restore automation, and
measured RPO/RTO from recurring exercises.

## Phase 67 — Synchronization engine

**Status:** Mailbox-authorized PostgreSQL change journal reads and sync
projection triggers implemented; IMAP multi-device synchronization remains
pending.

**Implemented:** Message flags, folder moves, deletes, labels, draft revisions,
and custom-property updates are represented in the durable change journal.
Cursor reads have bounded pages and permission checks.

**Remaining implementation:** Cursor invalidation/reset, compaction, IMAP
CONDSTORE/QRESYNC, protocol clients, backpressure, and interoperability/load
evidence.

## Phase 68 — Offline mail

**Status:** In-memory offline reference now applies remote move/flag metadata
to clean records and preserves dirty local records.

**Implemented:** Acknowledge-through semantics preserve operations queued
while an earlier batch is in flight.

**Remaining implementation:** Durable encrypted client cache, persistent
operation IDs, retry/dead-letter behavior, conflict resolution, and client
crash-recovery tests. The Rust reference queue is not a production client
sync store.

## Phase 69 — Storage optimization

**Status:** Attachment compression/deduplication prototype is in-memory;
durable message-body optimization is not implemented.

**Remaining implementation:** Object-store backend, versioned storage codecs,
tenant-isolated persistent deduplication, reference-safe garbage collection,
archival workers, and representative workload/recovery validation.

## Phase 70 — Mailbox platform integration

**Status:** Cross-component PostgreSQL foundations and isolated tests exist;
production integration is incomplete.

**Remaining implementation:** The Go API is still in-memory and is not wired
to the PostgreSQL repository. SMTP/IMAP service adapters, object storage,
outbox/retention workers, production identity, centralized telemetry, and
load/failover/restore evidence are still required. No production-readiness or
Brevo/Google-scale claim is made.
