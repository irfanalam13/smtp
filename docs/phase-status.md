# Phase implementation status and deferred work

This register is the authoritative cross-phase record of what is implemented,
what remains, why it is not implemented yet, and when the remaining work should
be applied. “Deferred” does not mean forgotten: each item has an owner phase or
an explicit prerequisite.

## Backend platform milestone M00 — audit and baseline

**Status:** Complete for audit scope; this is not a production-readiness claim.

The current executable wiring, feature classifications, prioritized
defect/security register, SMTP capability check, and baseline validation
results are recorded in the [M00 backend platform audit](backend-platform-audit.md).
M01 has since implemented and tested ingress STARTTLS, TLS-only authenticated
submission, tenant-scoped credential verification, connection throttling, and
graceful session draining. The remaining queue-worker delivery gap is tracked
under its own milestone; it is not part of M01.

For the connected handoff analysis and the completion boundary through Phase
10, see the [Phase 01–10 integration review](phase-integration.md).

The repository is intentionally a development and validation foundation. A
feature must not be described as production-ready merely because its
architecture, interface, prototype, or documentation exists.

## Backend platform milestone M02 — SMTP protocol conformance and ESMTP profile

**Status:** Complete for the selected bounded SMTP/ESMTP profile.

The parser now enforces visible-ASCII command syntax, conservative dot-atom
paths, strict CRLF, duplicate/unknown parameter rejection, ESMTP-only parameter
negotiation, correct DATA dot transparency, and transaction reset after
recoverable failures. EHLO exposes only configured `SIZE`, opt-in `8BITMIME`,
TLS-available `STARTTLS`, and post-TLS `AUTH`. Declared and streamed message
sizes are bounded by the global limit and, for authenticated submission, the
persisted tenant and verified-sender-domain limits. 8BITMIME preserves octets
only in declared message bodies; headers remain ASCII.

The parser/TCP/PostgreSQL test suites cover the supported profile and its
failure/recovery cases. PIPELINING, CHUNKING/BINARYMIME, SMTPUTF8, and DSN
remain unadvertised and are deferred until their response-ordering, storage,
Unicode, or recipient-delivery prerequisites are implemented.

## Backend platform milestone M03 — canonical message and MIME pipeline

**Status:** Bounded RFC 5322/MIME profile implemented and connected to the
M05 queue-to-SMTP worker; the PostgreSQL-to-SMTP acceptance path has a
disposable-database end-to-end test.

SMTP, queue, inbound storage, mailbox append, and API-generated submissions
now use schema-versioned message metadata while keeping envelope sender and
recipients separate from RFC 5322 headers. Accepted raw bytes are stored
unchanged. Queue, inbound, and mailbox rows persist parsed metadata under the
same versioned JSONB contract; mailbox search/render projections are derived
from the bounded metadata.

The Rust `mail-parser` pipeline used by SMTP and mailbox storage covers folded
and encoded headers, nested multipart bodies, transfer decoding, RFC 2231
filenames, inline parts, and malformed-encoding warnings. API submissions are
generated in a constrained text/plain profile and their metadata is derived
from the generated raw message. The parser bounds raw input to an absolute 10 MiB limit,
headers to 256 KiB and 200 fields (including nested messages), nesting depth
to 16, parts to 1,000, attachments to 100, decoded content to 20 MiB and a
20:1 aggregate expansion ratio, a two-second parsing deadline, and indexed
text to 256 KiB. Decoded lengths and attachment counts are checked before
transfer-decoding allocations. SMTP and mailbox callers share a four-worker
parsing semaphore; timed-out blocking work retains its slot until it exits.
Persisted metadata is capped at 768 KiB and database JSONB at 1 MiB. SMTP and
queue configuration enforce the same 10 MiB ceiling as the raw-message
storage constraints. Mailbox stores also apply this absolute ceiling before
copying input; structurally invalid but size-bounded APPEND data is retained
unchanged with an explicit parse-warning projection, while resource-limit
failures reject the append.

The PostgreSQL migration, queue/inbound/mailbox/API persistence, encoded and
folded headers, nested MIME bounds, decoded-size preflight, API metadata
derived from generated raw messages, and raw-byte preservation have focused
tests. The outbound worker now reparses stored bytes using the persisted schema
version before delivery. A disposable PostgreSQL queue-to-SMTP fixture test
verifies that the canonical message reaches the SMTP peer and remote
acceptance is persisted. SMTPUTF8/IDNA, archive extraction, attachment
scanning, and broader mailbox projection/rebuild workflows remain out of this
milestone.

## Backend platform milestone M05 — outbound MTA delivery

**Status:** In progress; recipient state, domain routing and a real delivery
worker are implemented, but M05 is not production-ready or complete.

Recipient deliveries and attempt history are persisted by migration 000021.
The worker claims domain batches with database fencing, retries only
temporary-failure recipients, records accepted/permanent/uncertain outcomes,
and reparses the canonical stored message before transmission. The client
routes by normalized domain, uses an exact-domain allowlist and validated IP
pins, handles implicit/null MX semantics, and does not retry ambiguous
post-DATA sends.

The worker now renews fenced recipient and domain leases while an SMTP batch
runs; failed renewal stops processing, and processor errors persist uncertain
outcomes rather than leaving work to an implicit expiry. Renewal and expiry
behavior are covered by PostgreSQL integration tests.

Remaining acceptance work includes tenant/domain TLS policy; IP-level
throttling and circuit breakers; delivery metrics and alerts; SMTP
interoperability testing; plus a review of resolver caching, destination policy
and operational rollout. MTA-STS/DANE, TLS reporting, connection reuse and
authenticated smarthost delivery remain unimplemented. Do not describe M05 as
complete or production-ready yet.

## Backend platform milestone M06 — inbound MTA and durable local acceptance

**Status:** Partially implemented; the bounded persistence path is wired, but
the full production inbound pipeline is not complete.

When `SMTP_INBOUND_ENABLED=true`, the SMTP executable requires PostgreSQL and
constructs the inbound acceptor. It routes recipients only to active mailboxes
on active tenants and verified domains. A message may fan out to multiple
mailboxes in one tenant. Cross-tenant batches are rejected with a generic
recipient error, and all copies plus mailbox-scoped recipient metadata commit
in one PostgreSQL transaction before SMTP `250`. Retries are detected with a
versioned, length-framed SHA-256 fingerprint and the stored sender, raw bytes,
and recipient set are checked before treating a match as a duplicate.

Additional M06 controls now include active primary and alias routing,
transactional mailbox quota reservation, bounded ClamAV INSTREAM and
SpamAssassin CHECK scanning, shared PostgreSQL per-IP/per-domain hourly rate
limits with HMAC-pseudonymized keys, persisted peer/listener/TLS metadata, and
one durable outbox event per accepted mailbox copy. Scanner outages fail closed
with a temporary SMTP response; malware and spam policy matches receive a
permanent rejection.

M06 remains incomplete for a production hosted-mailbox service: the SMTP
profile has no caller idempotency token, so identical intentional
resubmissions can be indistinguishable from retries. The outbox has no inbound
consumer, retention and object storage are missing, and listener metrics are
process-local. SPF/DKIM/DMARC are now evaluated under M07 and their policy
dispositions are integrated with durable inbound acceptance. See the
[inbound design](architecture/inbound-mail.md) and
[configuration reference](configuration.md).

## Backend platform milestone M07 — DNS, SPF, DKIM, DMARC, and authentication

**Status:** Core authentication and aggregate report generation/ingestion are
implemented; managed key lifecycle controls and full conformance evidence
remain incomplete.

Inbound DNS evaluation uses the `mail-auth` implementation with the configured
system resolver, DNSSEC validation, one concurrent upstream query, three-second
query timeout, a 15-second whole-message deadline, bounded positive/negative
TTL caches, CNAME handling, RFC 7208 SPF lookup limits, RFC 6376/RFC 8301/RFC
8463 DKIM verification, and RFC 9989 DMARC policy discovery/alignment. At most
four DKIM signatures and eight report destinations are evaluated per message.
RSA-SHA1 results are excluded from DMARC alignment. A DMARC DNS temporary error defers the SMTP transaction;
`p=reject` fails with SMTP `554`, and `p=quarantine` stores the message with a
quarantine status and outbox event. Safe result tokens are stored with the raw
message. The legacy simplified authentication helpers are not used for
enforcement.

Outbound real SMTP delivery requires a secret-manager projected DKIM key root.
The current signer supports Ed25519-SHA256 keys in PKCS#8 DER files with
restrictive permissions, and selects a key by the single From domain and
`active-selector`. The `smtp-dkim-keygen` binary creates a key and prints the
DNS TXT record. The `dkim-keyctl` binary activates a selector only after the
DNSSEC-validated public key matches the private key and atomically retires
inactive selectors after confirmed DNS removal and TTL expiry.

RFC 9990 report data is captured without message bodies or recipient addresses,
aggregated per UTC day and policy configuration, compressed as gzip XML, checked
against authorized `rua` destinations and size limits at receipt and again
before dispatch, persisted in an idempotent outbox, and sent through the
standard outbound queue. Received
aggregate XML/gzip can be validated and deduplicated by the ingestion command;
normalized report data expires after 90 days. Forensic reports remain disabled.

The remaining M07 technical work is tenant control-plane integration for key
lifecycle/configuration, explicit DNS query-count budget
accounting across SPF/DKIM/DMARC (the implementation currently relies on the
protocol limits, four-signature/eight-destination caps, serial DNS requests and
the 15-second deadline), outbound signing interoperability fixtures,
RFC-indexed conformance vectors, and real fixture-domain evidence. ARC/BIMI and
MTA-STS/TLS-RPT/DANE remain unsupported until the base conformance work is
complete. M07 is not yet complete.

## Backend platform milestone M08 — mailbox storage integration and administration

**Status:** Partial vertical slice implemented; hosted-mailbox production
acceptance remains incomplete.

**Implemented:** The Go API authenticates user-bound principals and proxies
mailbox calls to the Rust `mailbox-service` using a short-lived HMAC-signed
principal bound to HTTP method, URI, body digest, and append idempotency key.
The public API now reaches PostgreSQL-backed mailbox provisioning and reads,
aliases, mailbox permissions, folder listing/create/rename/delete, message
append/copy/move/flags/delete/expunge, labels, and quota reads/updates.
Mailbox authorization uses active user membership plus mailbox ownership or
an explicit mailbox grant; tenant owner/admin roles do not implicitly confer
message access. The API uses separate `mailbox:read`, `mailbox:write`, and
`mailbox:manage` scopes. Message append is transactionally idempotent for 30
days, rejects key reuse with a different payload, and charges quota/allocates
UID in the append transaction. Message content reads are audited. Same-folder
move is a no-op and does not consume UID space. Folder deletion protects root
INBOX and all special-use folders.

Migration `000026` adds scoped append-idempotency records, a 30-day expiry, and
an expiry index. Expired records are pruned in bounded batches during append
and by a scheduled mailbox-service maintenance loop, including for quiet
mailboxes. Rollback drops only this new idempotency table; it does not alter
mailbox messages or existing customer data.

Inbound accepted and quarantined outbox events are projected into mailbox
messages by the Rust service. Projection allocates the target folder UID,
inserts the message, and acknowledges the event in one transaction; failed
events use persisted exponential retry delays. SMTP acceptance has already
reserved the logical mailbox quota, so projection does not charge it again.
The service integration test covers inbox/Junk routing, retries, folder
creation, append replay, quota accounting, content reads, and audit records.

**Remaining implementation:** Add integration coverage for lifecycle/
permissions and concurrent lifecycle behavior, object storage and cleanup,
durable synchronization operations, and backup/restore plus operational
failure/recovery evidence. Outbound sent-copy/drafts projection remains
outstanding. The inbound worker does not provide generic event publication or
production monitoring. IMAP/POP3 protocol integration remains M09/M10.

**Reason and application point:** The current M08 slice uses PostgreSQL inline
message bytes and has no deployment identity provider for Go-to-Rust calls
beyond the shared HMAC key. It is a useful authenticated integration boundary,
not a hosted-mailbox or production-scale claim.

## Phase 11 — Email protocol engine

**Status:** Foundation research, SMTP conformance profile, and bounded
canonical message parsing implemented; remaining end-to-end extensions and
mailbox protocol work remain in progress.

**Implemented:** The [Phase 11 protocol-engine document](standards/phase-11-email-protocol-engine.md)
records the RFC-backed message layers, lifecycle, acceptance boundary, and
deferred milestones. Queue validation now preserves the distinction between
SMTP envelope paths and RFC 5322 message content, including support for the
null reverse-path used by generated mail.

**Remaining implementation:** SMTPUTF8/IDNA, DKIM/DSN, and any selected
mailbox protocol. Authenticated submission is implemented under M01; bounded
message parsing is implemented under M03.

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

**Remaining implementation:** DNS and destination policy,
external-delivery enablement, PIPELINING, DSN, SMTPUTF8/REQUIRETLS, and
interoperability testing are separate production hardening milestones.

## Phase 13 — SMTP session state machine

**Status:** Bounded submission state machine, STARTTLS reset, and TLS-backed
AUTH PLAIN flow implemented and tested.

**Implemented:** The session owns explicit connection, greeting,
authentication, envelope, recipient, DATA, and closed states. `RSET` preserves
authentication, AUTH continuation is bounded, credentials are delegated to a
tenant-scoped PostgreSQL verifier, and AUTH is advertised only after TLS.

**Remaining implementation:** Richer recipient policy and broader
interoperability coverage beyond the M01 Python `smtplib` acceptance test.

## Phase 14 — SMTP security

**Status:** Complete for the M01 specification and acceptance criteria.

**Implemented:** Certificate-backed Rustls server configuration, optional
implicit TLS, in-place STARTTLS with SMTP state reset, TLS-only AUTH PLAIN,
tenant-scoped Argon2id PostgreSQL credentials with expiry/rotation/revocation,
verified-domain sender authorization, authentication throttling, explicit TLS
failure handling, and graceful session draining.

**Additional operational controls:** Tenant-scoped credential list endpoints
require `smtp:credentials:read`; create/rotate/revoke endpoints
require `smtp:credentials:write`. Generated passwords are returned once,
expiry is enforced, and changes are audited transactionally.
PostgreSQL-backed IP/tenant/account throttling is shared across ingress
instances using a deployment-injected HMAC key. Mounted TLS certificate/key
files reload every 30 seconds; invalid rotations retain the last valid
configuration.

**Acceptance evidence:** The SMTP TCP suite verifies plaintext AUTH rejection,
STARTTLS state reset, invalid credentials, no TLS downgrade after handshake
failure, pipelined plaintext rejection, connection exhaustion, slow-client
deadlines, and graceful shutdown. Database-backed tests verify credential
hashing, wrong-password/wrong-tenant/expired/revoked behavior, shared throttles,
sender authorization, and a complete Python `smtplib` STARTTLS/AUTH/submission
transaction. The CI PostgreSQL job runs the migration, Rust integration suite,
and Go credential lifecycle integration test.

**Beyond M01:** Wider client-matrix testing, direct cloud secret-manager API
integration (deployment-managed secret mounts and environment injection are
supported), client certificates, OCSP/stapling policy, and outbound delivery
worker integration remain separate deployment or product work. M01's secure
submission acceptance does not mean queued messages are delivered or that the
whole platform is production-ready.

## Phase 15 — SMTP extensions (ESMTP)

**Status:** Bounded `SIZE` and opt-in `8BITMIME` implemented and tested.

**Implemented:** Runtime-gated EHLO capabilities, strict `MAIL FROM` ESMTP
parameter validation, global/tenant/domain size enforcement, and body-only
8-bit transfer with raw-octet preservation.

**Deferred by profile:** `PIPELINING`, `CHUNKING`/`BDAT`, SMTPUTF8/Unicode
storage and delivery, and DSN parameters/reports remain unadvertised until
their end-to-end prerequisites are implemented.

## Phases 16–20 — Queue, delivery, inbound, MIME, and message parsing

The phase-specific contracts and current boundaries are documented under
[`standards/`](standards/). Phase 16 has received a schema correction for the
SMTP null reverse-path. M03 now connects the bounded canonical parser to SMTP
queueing, inbound acceptance, mailbox append, and API-generated submissions.
This does not complete queue worker outcomes, external delivery, inbound
routing, or attachment scanning. The existing queue, delivery, and inbound
components remain bounded foundations, not claims of Brevo/Google-scale
production readiness.

## Phases 21–25 — Mailbox protocols and storage

Research and storage contracts are documented under [`standards/`](standards/).
Phase 21/25 now has a durable folder/message schema and shared bounded mailbox
model. IMAP remains pending and must be built on this storage contract rather
than independent in-memory state. POP3 is explicitly out of scope for the
first production release; `smtp-pop3` contains parser/state-machine fixtures
only and has no listener or mailbox implementation.

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
durability, queue lease semantics, outbound destination safety, inbound
acceptance design, reliability assumptions, security boundaries, and
extension paths are documented under [`architecture/phase-01/`](architecture/phase-01/).

**Remaining implementation:**

- Complete PostgreSQL coverage for remaining control-plane operations;
  selected tenant/domain, API-key, submission, and mailbox paths are already
  database-backed.
- Choose the production deployment/runtime and service topology.
- Define the production event publication adapter and delivery read model.
- Extend verified-domain sender authorization with bounce/suppression and
  abuse-control boundaries at the send API.
- Decide whether hosted mailboxes and inbound reception are launch features.

**Reason and application point:** Phase 01 intentionally established boundaries
without implementing application services or locking in unvalidated cloud
choices. Complete remaining repository and deployment work in Phases 06, 08,
09, and 10; apply hosted-mailbox extensions only after a product decision and
dedicated protocol/storage milestones.

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

- Complete PostgreSQL-backed control-plane state; development identity and
  usage/quota state remain process-local, while several core API paths already
  use durable repositories.
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

**Status:** `smtp-auth` authorization/secret primitives, optional OIDC API
resource-server verification, generic OIDC browser login, and PostgreSQL
browser sessions, and self-service email/password account registration,
verification, login, and recovery are implemented. OIDC provisioning and
several production integrations remain deferred.

**Implemented:** Typed human/service identities, default-deny authorization,
resource tenant checks, scoped API-key verification, constant-time comparison,
expiry/revocation checks, CSPRNG identifiers and secrets, development-mode
production guard, audit safety guidance, threat modeling, incident guidance,
and security regression tests.

**Remaining implementation:**

- Configure and operationalize OIDC providers; implement OIDC user
  provisioning, MFA/passkeys, invitations/account linking, GitHub's OAuth
  identity API, and per-tenant provider management.
- Add JWT access/refresh-token lifecycle only when a defined API client needs
  bearer refresh tokens; current browser authentication uses revocable opaque
  sessions.
- Complete PostgreSQL persistence for audit events and remaining identity
  state; API-key verifier persistence is already implemented for durable API
  principals.
- Integrate AWS Secrets Manager, KMS, certificate lifecycle, and key rotation.
- Establish EKS workload identity or mTLS for internal service calls.
- Add durable audit export, alerting, access review, and incident drills.
- Replace process-local authentication rate limits with a distributed
  deployment-backed limiter and verify provider interoperability.
- Resolve the dependency advisories documented by the security workflow.

**Reason and application point:** Keycloak, AWS KMS/Secrets Manager, and EKS
are selected, but no realm, AWS account/IAM, cluster, or operational ownership
is configured. Applying provider integrations without those trust boundaries
would be misleading and unsafe. Complete configuration and validation before
public launch.

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

- Complete PostgreSQL-backed API coverage for the remaining development-store
  operations and durable audit/quota state; selected core API routes already
  use PostgreSQL repositories.
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

**Reason and application point:** The current API has one Go HTTP consumer and retains an in-memory development
identity/store path and process-local usage counters. Adding gRPC, a generic
client, process-local idempotency, or a fake broker would create misleading guarantees.
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
- Complete PostgreSQL-backed control-plane coverage, including development
  identity and usage/quota state that remain process-local.
- Complete Keycloak user/session provisioning, AWS Secrets Manager/KMS,
  certificate management, and EKS service identity.
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
enforces tenant-consistent mailbox/domain references. The Go API now has
PostgreSQL-backed mailbox listing and bounded message metadata reads using
durable API keys and tenant/mailbox authorization. User-bound keys require
mailbox ownership or an explicit permission; legacy tenant-only keys remain
usable for existing scopes but are explicitly denied mailbox access.

**Remaining implementation:** Authenticated mailbox write/lifecycle APIs,
deployable object storage, encryption and key lifecycle, capacity/partition
strategy backed by workload evidence, disaster recovery, and load/failover
validation.

**Reason and application point:** Selected mailbox reads are PostgreSQL-backed,
but mailbox writes, development identity/usage state, and message-byte object
storage remain incomplete. Apply the architecture incrementally in Phases
42–45; do not enable hosted mailbox traffic until storage, auth, retention,
operations, and recovery criteria are evidenced.

## Phase 42 — User mailboxes

**Status:** Database schema foundation implemented; lifecycle and API
operations are not yet wired.

**Implemented:** Mailbox type, optional tenant-member owner, quota fields,
lifecycle statuses, same-tenant aliases, and mailbox-scoped permissions are
added in migration `000008`. Rust storage primitives provide transactional
mailbox provisioning with default folders, lifecycle transitions, alias
creation, and permission grant/revocation.

**Remaining implementation:** Authenticated mailbox provisioning, idempotent
alias and permission APIs, audited lifecycle transitions, inbound routing to
mailbox IDs, retention-aware deletion, and durable control-plane write
handlers. The read-only mailbox list/message routes are now PostgreSQL-backed.

**Reason and application point:** Mailbox reads now use durable PostgreSQL
repositories, but mailbox writes and lifecycle operations are not exposed
through the Go API. Durable domain operations now use PostgreSQL; usage
counters and development identity remain process-local.
Build authenticated writes on the durable mailbox schema and explicit
owner/permission authorization rather than layering them over ephemeral state.

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

**Status:** Bounded MIME parsing and SMTP/inbound/mailbox integration
implemented and tested.

**Implemented:** A standards-oriented parser extracts encoded subject and
addresses, nested plain/HTML bodies, embedded images, attachments, message
references, and decode-warning counts under explicit input, nesting, part,
attachment, and decoded-size limits.

**Remaining implementation:** Quarantine/error policy, scanner handoff,
extensive malformed/fuzz/RFC fixtures, and safe HTML rendering.

**Reason and application point:** A parser projection must be attached to a
durable message transaction without modifying the canonical RFC message.
MIME indexing is limited to bounded metadata fields; rendering must still
treat HTML bodies as untrusted input.

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

**Implemented:** The authenticated HTTP submission endpoint now atomically
enqueues bounded plain-text messages and recipients with idempotent retries.
The dashboard connects through a same-origin API proxy and displays queue
states.

**Remaining implementation:** SMTP submission and sent-copy transaction,
queue/DSN event consumer, recipient-data retention, external delivery
synchronization, and end-to-end delivery tests. The local queue worker remains
deterministic and does not send messages externally.

## Phase 51–60 integration boundary

The Go API now exposes authenticated PostgreSQL-backed mailbox listing and
bounded message metadata reads. User-bound keys require an active tenant
membership and mailbox ownership or an explicit permission. Existing
tenant-only keys remain usable within their existing scopes but are denied
mailbox access. PostgreSQL integration tests verify this compatibility and
authorization boundary against an isolated database. Mailbox write workflows
are not exposed through the API, and inbound/outbound services do not yet
invoke these workflows. The outbox is a durable lease/ack/retry primitive, not
a running broker worker. No claim of production readiness, service SLO, load
capacity, disaster-recovery evidence, or compliance certification is made.

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

**Implemented:** Go API mailbox list/message reads authenticate PostgreSQL
API keys bound to active tenant users and enforce mailbox owner/permission
checks in tenant-scoped SQL. Startup requires PostgreSQL, readiness pings the
database, and shutdown closes the pool. Other control-plane state remains
in-memory.

**Remaining implementation:** OIDC/JWT integration, durable domain/control
plane writes, SMTP/IMAP service adapters, object storage, outbox/retention
workers, centralized telemetry, and load/failover/restore evidence remain
required. No production-readiness or Brevo/Google-scale claim is made.

## Phases 71–80 — SaaS platform and developer APIs

**Status:** Research and phase design documents created; Phase 72 has a first
database-isolation change, with implementation and full validation pending.

**Implemented:** The repository already has a PostgreSQL-backed tenant and
membership foundation, a Go control plane, mailbox-scoped authorization,
REST/OpenAPI conventions, reserved SDK/CLI packages, and documented
control/data-plane separation. These are foundations, not a complete SaaS
platform.

**Recommended architecture:** Reuse `tenants` as the organization/security
boundary; use shared PostgreSQL with explicit tenant ownership and layered
isolation as the initial default; consider workspaces only where delegated
resource segmentation is needed. Use external OIDC identity, a bounded
tenant-scoped permission model, provider-neutral asynchronous usage metering,
managed edge protection, REST/OpenAPI as the canonical API, and generated
SDKs. GraphQL remains additive and requires demonstrated product need.

**Initial implementation:** Phase 72 migration `000014` prevents an inbound
message from referencing a mailbox owned by another tenant; an opt-in
PostgreSQL integration test covers the constraint. This is not a full tenant
isolation guarantee.

**Remaining implementation:** Phases 71–80 are documented individually under
[`standards/`](standards/). Work must proceed in dependency order: approve
architecture and tenant boundaries; implement/test data isolation; add
organization/workspace lifecycle; production identity; RBAC; billing; edge
gateway; durable REST resources; then decide/implement GraphQL and publish
SDKs. Phases 81–100 have individual design documents; their implementation
status follows below.

**Why this remains open:** The repo has tenant, database, API, and security
foundations but lacks production identity, general RBAC, organization/workspace
workflows, billing policy/provider, deployment edge, and complete APIs. The
recommended tenancy strategy still requires a reviewed architecture decision
and comprehensive cross-tenant tests. Production readiness also requires
deployment, operational, security, load, failure, and recovery evidence that
cannot be established by source changes alone.

## Phases 81–90 — Webhooks, eventing, templates, and compliance platform

**Status:** Phase design documents added; Phase 81 has an isolated signing
primitive; delivery/integration implementation remains pending.

**Implemented:** The repository provides a PostgreSQL mailbox outbox with
lease/ack/retry primitives, typed event envelopes, bounded mailbox filters
and rules, a limited PostgreSQL mailbox search foundation, and an audit table.
Phase 81 adds a Standard Webhooks-compatible HMAC signer in the Go API; it is
not yet connected to endpoint secrets or outbound delivery.

**Remaining implementation:** Phase 81 endpoint management, KMS-backed
secrets, SSRF-safe dispatcher, retries/DLQ/replay; Phase 82 general event
schemas, consumers, and measured broker decision; Phase 83 tenant templates
and secure rendering; Phase 84 durable workflow/scheduler; Phase 85 prioritized
provider integrations; Phase 86 resumable migration connectors; Phase 87
global search and projection freshness; Phase 88 durable analytics; Phase 89
scheduled/report exports; Phase 90 durable audit, privacy workflows, and
applicable compliance evidence. Individual contracts and rationale are in
[`standards/phase-81-webhook-system.md`](standards/phase-81-webhook-system.md)
through [`standards/phase-90-audit-compliance.md`](standards/phase-90-audit-compliance.md).

**Why this remains open:** Existing primitives are not production services:
there are no running event/webhook consumers, KMS-backed webhook secrets,
template/automation APIs, provider adapters, global search service, analytics
warehouse, or report scheduler. Audit/compliance also requires operational
controls and, where applicable, legal review and independent assurance.
Source-level foundations alone cannot establish reliability, compliance, or
Brevo/Google-scale capacity.

## Phases 91–100 — White-label, extensions, administration, and enterprise release

**Status:** Phase design documents added; Phase 91 has deployment-level
dashboard branding; tenant-specific white-labeling and all other phase
implementations remain pending.

**Implemented:** The dashboard brand label and page metadata accept the
public build-time `NEXT_PUBLIC_BRAND_NAME` setting, defaulting to `SMTP
Platform`. This is deployment-wide branding only. The repository has no
tenant hostname verification, custom login experience, tenant theme storage,
or logo asset pipeline.

**Remaining implementation:** Phase 91 verified custom application domains,
tenant branding, safe asset management, and branded authentication; Phase 92
reviewed/sandboxed extensions; Phase 93 durable tenant admin APIs/UI; Phase 94
separate privileged operator access and support console; Phase 95 privacy-safe
AI boundaries; Phase 96 preference-aware notification delivery; Phase 97
support workflows; Phase 98 production-like availability/capacity/failover;
Phase 99 security and resilience evidence; Phase 100 an actual scoped
production release. Detailed contracts and rationale are in
[`standards/phase-91-white-label-platform.md`](standards/phase-91-white-label-platform.md)
through [`standards/phase-100-enterprise-saas-release.md`](standards/phase-100-enterprise-saas-release.md).

**Why this remains open:** Phases 71–90 are not complete, and this phase set
depends on their tenant isolation, durable APIs, production identity, audit,
events, object storage, and operational controls. Building tenant-specific
branding, global administration, marketplace code execution, AI processing,
or support impersonation before those controls exist would create avoidable
cross-tenant and sensitive-data risks. Production scalability and enterprise
release also require a selected deployment target and external test/evidence.

## Phases 101–110 — Enterprise security and high availability

**Status:** Design documents added; initial code slices exist for Phases 101,
102, 103, 105, and 110. Provider targets are selected for Keycloak OIDC, AWS
KMS/Secrets Manager, and AWS EKS, but the latter two are not integrated or
deployed. The phase set is not complete.

**Initial implementation:** The API rejects the development identity adapter
unless `APP_ENVIRONMENT` explicitly identifies development. Optional Keycloak
OIDC access tokens are validated for signature, exact issuer, API audience,
expiry, Keycloak bearer-token type, and bounded size; the issuer/subject must map to a pre-provisioned
active PostgreSQL user and membership. Tenant selection is checked against
that membership, and current fixed role scopes come from PostgreSQL, not token
claims. Migration `000015` adds the identity issuer field. API responses have
defensive browser/cache headers and bounded request headers. During graceful
shutdown, readiness is withdrawn before the listener drains. Go tests cover
the OIDC verifier and these safeguards; PostgreSQL-backed membership tests
require an isolated database. Migration `000028` adds browser sessions with
OIDC Authorization Code/PKCE, server-managed opaque session cookies, CSRF and
same-origin checks, idle/absolute expiry, device revocation, and logout-all.
Go vulnerability scanning is enforced in the security workflow; after
upgrading to Go 1.26.10 and `golang.org/x/text` 0.41.0, `govulncheck` reported
no known vulnerabilities. This does not provide user provisioning, MFA policy
enforcement, full RBAC/ABAC administration, refresh tokens, workload identity,
managed secrets, HSM, durable audit export, or clustered HA.

**Remaining implementation:** Phase 101 EKS workload identity and service
trust; Phase 102 user provisioning/deprovisioning, browser OIDC/session
lifecycle, MFA configuration evidence, and optional SAML; Phase 103 a reviewed
complete role/policy matrix; Phase 104 AWS IAM roles, Secrets Manager, KMS,
certificate lifecycle, and key-rotation/recovery tests; Phase 105 durable
security audit and signed/SBOM release supply chain; Phase 106 actual EKS
cluster/network/load-balancer deployment; Phase 107 production distributed
mail protocol services; Phase 108 managed storage HA and restore; Phase 109
safe clustered queue operation; Phase 110 production failover, rollouts,
self-healing, SLOs, and recovery exercises. Detailed contracts, remaining
rationale, and acceptance criteria are in
[`standards/phase-101-zero-trust-service-identity.md`](standards/phase-101-zero-trust-service-identity.md)
through [`standards/phase-110-failover-rollouts-self-healing.md`](standards/phase-110-failover-rollouts-self-healing.md).

**Why this remains open:** The repository explicitly does not show Phases
71–100 complete. Keycloak, AWS KMS/Secrets Manager, and EKS are selected, but
the realm, AWS accounts/IAM, cluster, managed data services, on-call ownership,
and failover evidence are not present. These change security boundaries and
durability semantics, so they cannot be represented as finished by generic
code changes.

## Phases 111–120 — Scalability, performance, and observability

**Status:** Phase designs added; a bounded API metrics enhancement is
implemented; distributed scaling and centralized operations remain pending.

**Implemented:** The Go API's process-local Prometheus exposition now includes
a fixed-bucket HTTP request-duration histogram in seconds and an active-request
gauge. It retains existing aggregate counters and status-class metrics. Labels
do not include tenant IDs, request IDs, email addresses, message IDs, or raw
paths. Focused tests cover active-request lifecycle, cumulative histogram
buckets, and metric output.

**Remaining implementation:** Phase 111 approved SLOs, representative workload
profiles, and repeatable capacity evidence; Phase 112 measured cache use cases
and a tenant-safe distributed cache; Phase 113 data-placement, partitioning,
and shard migration design; Phase 114 horizontally operated queue workers and
backpressure; Phase 115 fleet-wide rate limiting, safe autoscaling, and
load/benchmark evidence; Phase 116 central metrics/log collection and
retention; Phase 117 end-to-end trace propagation and export; Phase 118
production dashboards, alerting, and external uptime checks; Phase 119
centralized SMTP, mailbox, storage, and queue SLIs; Phase 120 historical
capacity forecasting and validated anomaly detection. The detailed contracts,
remaining rationale, and acceptance criteria are in
[`standards/phase-111-scalability-architecture.md`](standards/phase-111-scalability-architecture.md)
through
[`standards/phase-120-capacity-planning-and-anomaly-detection.md`](standards/phase-120-capacity-planning-and-anomaly-detection.md).

**Why this remains open:** The metrics are process-local, reset on restart, and
not centrally aggregated. There is no production deployment, representative
load evidence, multi-instance service operation, telemetry backend, alert
ownership, or historical dataset. Phases 71–100 are not complete, and
Phases 101–110 remain partial; tenant, identity, storage, queue, and HA
contracts are prerequisites for several of these capabilities. Caching,
sharding, distributed throttling, autoscaling, and automated anomaly response
change consistency and failure behavior and require operational evidence.

## Phases 121–130 — Disaster recovery, compliance, and enterprise governance

**Status:** Phase designs added; local PostgreSQL backup checksum generation
and verification are implemented; production recovery and governance remain
pending.

**Implemented:** The local PostgreSQL backup script writes a SHA-256 sidecar
after a successful logical dump. `make verify-postgres-backup BACKUP=...`
verifies that the dump matches its sidecar. This detects accidental
modification or corruption only; it is not an authenticated manifest, a
successful restore test, point-in-time recovery, a protected backup service,
or cross-region disaster recovery.

**Remaining implementation:** Phase 121 inventory, protected backup
automation, restore validation; Phase 122 approved data-residency and
cross-region replication; Phase 123 base backups/WAL archiving and measured
PITR; Phase 124 fenced failover/failback and geo-redundant service; Phase 125
business-owned, recurring recovery and continuity exercises; Phase 126
framework applicability assessments and governance evidence; Phase 127
legally reviewed retention, legal hold, and eDiscovery workflows; Phase 128
durable tamper-evident audit and controlled evidence exports; Phase 129
tenant-scoped enterprise administration and lifecycle; Phase 130 stable
enterprise APIs and operational governance. Detailed contracts, remaining
rationale, and acceptance criteria are in
[`standards/phase-121-backup-integrity-and-recovery-inventory.md`](standards/phase-121-backup-integrity-and-recovery-inventory.md)
through
[`standards/phase-130-enterprise-apis-and-operational-governance.md`](standards/phase-130-enterprise-apis-and-operational-governance.md).

**Why this remains open:** No production backup target, WAL archive, region,
key-recovery procedure, tested RPO/RTO, or continuity team is configured.
Compliance applicability depends on real customer data, geography, contracts,
and organizational roles; legal holds and eDiscovery also require legal
process and strict authorization. Phases 71–120 are not complete in the
repository, while Phases 101–110 remain partial, so enterprise administration
and compliance workflows must not be presented as complete or certified.

## Phases 131–140 — AI-assisted email analysis and organization

**Status:** Phase designs added; offline aggregate email-feature extraction is
implemented; no AI model, provider, or automated mail decision is implemented.

**Implemented:** The Rust shared-types crate exposes
`extract_email_features` over the existing bounded MIME projection. It returns
aggregate text/link/attachment/parser counts only and does not include raw
addresses, subjects, filenames, URLs, body text, or attachment bytes. It makes
no network requests, does not execute attachments, and does not produce a
spam, phishing, malware, classification, or priority verdict. Unit tests cover
counts, executable-like filename/media-type indicators, and absence of raw
message values from the feature output.

**Remaining implementation:** Phase 131 approved model gateway, privacy/data
processing boundaries, evaluation, and asynchronous inference; Phase 132
versioned feature schemas and production pipeline integration; Phase 133
evaluated spam scoring and reversible quarantine; Phase 134 phishing
signals/reputation and isolated SSRF-safe URL analysis; Phase 135 a maintained
attachment scanner and isolated sandbox; Phase 136 privacy-reviewed link
reputation; Phase 137 opt-in email classification; Phase 138 personalized,
consent-based priority; Phase 139 reversible smart inbox views; Phase 140
source-linked, evaluated conversation summaries and action extraction.
Detailed scope, remaining rationale, and acceptance criteria are in
[`standards/phase-131-ai-platform-foundation.md`](standards/phase-131-ai-platform-foundation.md)
through
[`standards/phase-140-conversation-intelligence.md`](standards/phase-140-conversation-intelligence.md).

**Why this remains open:** The `smtp-ai` package is a placeholder, and there
is no approved model provider, consent/data-use policy, labeled evaluation
set, inference infrastructure, operational ownership, or retention boundary.
Email contains sensitive content and untrusted instructions. Phases 71–130
are also not complete in the repository; tenant isolation, durable storage,
audit, HA, and privacy controls are prerequisites. AI scores must remain
advisory until validated, and automated destructive or external actions must
require ordinary authorization and human confirmation.

## Phases 141–150 — AI assistance, language, and relationship insights

**Status:** Phase designs added; a bounded review-only text suggestion
contract is implemented; no AI inference, translation, entity, intent,
sentiment, task, calendar, or contact-intelligence feature is operational.

**Implemented:** The Rust shared-types crate now provides
`DraftSuggestion` for bounded reply, compose-assistance, summary, or translation
text with a kind, model-version provenance, and optional source message IDs.
Construction rejects empty or oversized content, unsupported control
characters, excessive/duplicate/nil source IDs, and invalid model-version
metadata. The value represents an untrusted, user-reviewable suggestion; it
does not authorize mailbox reads, sanitize markup, send mail, or invoke a
provider. Unit tests cover valid construction and rejection boundaries.

**Remaining implementation:** Phase 141 provider-backed reply generation and
review UI; Phase 142 safe draft editing and reversible composer operations;
Phase 143 source-linked summaries with factuality/omission evaluation; Phase
144 confidence-aware language detection and standards-based translation;
Phase 145 consented, access-controlled entity extraction and retention; Phase
146 evaluated intent taxonomy; Phase 147 privacy/fairness-reviewed sentiment
analysis; Phase 148 source-linked task candidates with explicit confirmation;
Phase 149 RFC 5545 calendar parsing and user-approved calendar integrations;
Phase 150 user-controlled, mailbox-scoped contact insights. Detailed
contracts, remaining rationale, and acceptance criteria are in
[`standards/phase-141-smart-reply-generation.md`](standards/phase-141-smart-reply-generation.md)
through
[`standards/phase-150-contact-relationship-intelligence.md`](standards/phase-150-contact-relationship-intelligence.md).

**Why this remains open:** There is no approved AI provider, opt-in and
retention policy, inference service, evaluation dataset, user interface, or
authorized calendar/contact integration. These features process sensitive
communications and may create consequential inferred data. Suggestions must
remain review-only; calendar/task/send actions require explicit user
confirmation and existing authorization. Earlier phases 71–140 are not
verified complete, and their tenant, privacy, audit, threading, and storage
controls are prerequisites.

## Phases 151–160 — Semantic retrieval, AI operations, and governance

**Status:** Phase designs added; a bounded in-memory cosine-ranking primitive
is implemented; embedding generation, vector storage, model operations, and
governed AI workflows remain pending.

**Implemented:** The shared Rust crate exposes `semantic_search` over
caller-supplied vectors. It bounds dimensions, candidate count, mailbox scope,
and result count; rejects non-finite/zero vectors and malformed authorized
candidates; filters by explicit tenant and mailbox IDs; produces deterministic
cosine ranking; and returns IDs/scores only. It does not generate embeddings,
load mail, persist vectors, or replace authorization. The caller must derive
the permitted mailbox scope from the authoritative identity/permission
system. Tests cover tenant/mailbox filtering, ranking/tie behavior, invalid
inputs, and unauthorized rows.

**Remaining implementation:** Phase 151 embedding model/index lifecycle and
authorization-aware durable retrieval; Phase 152 consented recommendation
pipeline; Phase 153 deterministic, auditable AI workflow execution; Phase 154
permission-aware assistant and explicitly approved tools; Phase 155 model/
prompt registry, evaluation gates, rollout and rollback; Phase 156 enforced
inference privacy, provider controls, isolation and audit; Phase 157 quality,
drift, latency and cost monitoring; Phase 158 consented, poisoning-resistant
feedback and reviewed training; Phase 159 organization-level AI governance;
Phase 160 production integration with stable interfaces and evidence.
Detailed contracts, remaining rationale, and acceptance criteria are in
[`standards/phase-151-ai-search-and-semantic-retrieval.md`](standards/phase-151-ai-search-and-semantic-retrieval.md)
through
[`standards/phase-160-unified-ai-intelligence-platform.md`](standards/phase-160-unified-ai-intelligence-platform.md).

**Why this remains open:** There is no embedding/model provider, vector
database, deployed AI runtime, authorized retrieval adapter, governance
policy, evaluation corpus, or production AI operations. The in-memory ranking
primitive is not semantic search until embeddings are produced and the index
is integrated with durable, permission-filtered mailbox retrieval. Phases
71–150 are not verified complete in the repository; tenant isolation, privacy,
audit, model governance, and user controls remain prerequisites.

## Phases 161–180 — Global infrastructure

**Status:** Phase designs added; a provider-neutral regional topology
configuration validator is implemented; no cloud infrastructure or
multi-region service is deployed.

**Implemented:** `shared-types::regional_topology` validates a proposed
primary/standby topology, unique bounded region and availability-zone
identifiers, and required service coverage. It is an offline configuration
guardrail only. It does not provision AWS/EKS, verify real regions or AZs,
check residency, perform health checks, route traffic, replicate data, or
trigger failover.

**Remaining implementation:** Phase 161 approved service/data topology and
regional SLOs; Phase 162 independent regional clusters and deployment
automation; Phase 163 managed DNS and mail-record operations; Phase 164
residency-safe geo routing; Phase 165 protocol-aware global load balancing;
Phase 166 hardened edge ingress; Phase 167 safe static/private CDN delivery;
Phase 168 encrypted object storage replication and reconciliation; Phase 169
fenced database replication; Phase 170 regional cache policy; Phase 171
durable distributed queues; Phase 172 authenticated service discovery; Phase
173 production DR workflows; Phase 174 proven failover/failback; Phase 175
isolated multi-region backup; Phase 176 resilience testing; Phase 177 measured
traffic optimization; Phase 178 verified regional data residency; Phase 179
central operations and alerting; and Phase 180 approved global production
validation. Detailed contracts, remaining rationale, and acceptance criteria
are in
[`standards/phase-161-global-infrastructure-architecture.md`](standards/phase-161-global-infrastructure-architecture.md)
through
[`standards/phase-180-global-production-validation.md`](standards/phase-180-global-production-validation.md).

**Why this remains open:** The immediate target is a localhost development
environment on the user's Ubuntu laptop; cloud infrastructure is intentionally
deferred because hosting budget and provider requirements are not available.
There is no AWS account, regions, clusters, network, DNS, edge endpoints,
managed database, object storage, global telemetry, or operations ownership
configured. Local development does not need these hosted capabilities.
Phases 71–160 are also not verified complete in the repository; tenant
isolation, data residency, audit, AI privacy, stateful HA, and recovery
contracts remain prerequisites. Multi-region mail is especially sensitive to
single-writer ownership, queue deduplication, SMTP reputation, DNS caching,
and regulatory placement; it cannot be claimed complete by topology code or
documents alone.
