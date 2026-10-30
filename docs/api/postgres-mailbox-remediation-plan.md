# PostgreSQL mailbox API remediation plan

This plan is based on the Go API, PostgreSQL migrations, security ADRs, and
Rust mailbox-store boundaries in this repository. Only milestone 1 is in scope
for the current change.

## Verified implementation map

- `GET /v1/mailboxes` and the message list/get routes are registered in
  `smtp-api/internal/controlplane/handler.go`. Mailbox reads use
  `PostgreSQLRepository`; durable domain operations and queue submissions also
  use PostgreSQL, while usage counters remain in the in-memory `Store`.
- `smtp-api/cmd/api/main.go` requires `DATABASE_URL`, creates and pings a
  bounded `pgxpool`, injects the PostgreSQL repository, and uses it for
  readiness.
- Migration `000004` defines tenant-scoped API keys with SHA-256 verifier
  hashes, prefix, scopes, expiry, last-use, and revocation. Migration `000012`
  adds a nullable creating-member reference. Migrations `000007` and `000008`
  define messages, mailbox owners, mailbox permissions, and same-tenant
  constraints.
- New API keys are user-bound. Authentication validates key hash, scope,
  expiry, revocation, active tenant, active user, and tenant membership.
  Mailbox queries further require tenant membership and ownership or an
  explicit `read`, `read_write`, or `manage` permission.
- The production identity provider remains undecided. ADRs `0006` and
  `phase-07/0001` explicitly defer OIDC/JWT until issuer, audience, and key
  discovery decisions are approved.
- `smtp-core/src/mailbox_store.rs` contains durable mailbox workflows and an
  outbox lease/ack primitive. `smtp-session/src/inbound.rs` contains an
  inbound store adapter. The Go API has no mailbox mutation routes; message
  submissions are persisted to the queue but are not externally delivered.

## Prioritized milestones

### 1. Preserve legacy key compatibility without granting mailbox access

**Status:** Implemented and verified in this change with Go tests and an
isolated PostgreSQL integration run.

- **Dependencies:** Existing migrations through `000012`; apply new migration
  `000013` before deploying the updated API.
- **Files:** Go PostgreSQL authentication and mailbox guards in
  `smtp-api/internal/controlplane/{postgres.go,handler.go}`; schema migration
  `smtp-storage/postgres/000013_classify_api_key_principals.{up,down}.sql`;
  integration tests and API/storage documentation.
- **Database change:** Add `api_keys.principal_type`. Backfill keys with a
  creator as `user`; classify pre-existing keys without one as `tenant`.
  Newly issued keys are explicitly `user`.
- **Rollout safety:** Apply migrations `000012` and `000013` in order with API
  key issuance paused, before deploying code that writes user-bound keys. If
  `000012` has already been used to issue keys and a membership may have been
  removed, audit creator-less keys before `000013`; the old nullable column
  cannot reconstruct that history. Before restarting the API, mark
  ambiguous/detached keys `principal_type='user'` (fail closed) unless their
  tenant-only origin is confirmed. Replace an ambiguous key if its owner
  cannot be safely restored.
- **Security:** Legacy tenant keys continue to authenticate only within their
  existing scopes and active tenant. They have no user identity and cannot
  list/read mailbox data or create user-bound credentials. User keys require
  an active owner membership; removal never converts a user key into a
  tenant-wide key. Mailbox access remains owner/permission based.
- **Tests:** Existing tenant keys authenticate for existing tenant-scoped
  endpoints; user-bound keys can read granted mailboxes; tenant-only keys are
  denied mailbox list/list-message/get; cross-tenant and missing-permission
  reads remain hidden; expired, revoked, removed-member, missing-scope, and
  development identities are rejected as applicable; API-key issuance
  persists a user principal.

### 2. Select and integrate the production human identity provider

**Status:** Blocked on product/security configuration; do not implement a
token verifier until decisions are approved.

- **Dependencies:** Select the issuer and tenant/organization model; define
  allowed audiences, accepted signing algorithms, JWKS discovery and cache
  behavior, claim-to-`external_subject` mapping, clock-skew policy, and key
  rotation/revocation expectations.
- **Files:** ADRs `docs/architecture/adr/0006-control-plane-auth-and-isolation.md`
  and `docs/architecture/adr/phase-07/0001-security-foundation-boundary.md`,
  Go authentication/principal middleware, configuration, OpenAPI security,
  and identity integration tests.
- **Database change:** The `users.external_subject` unique column exists.
  Issuer-namespaced subjects or tenant claims may require a forward migration;
  determine that only after the provider contract is chosen.
- **Security:** Validate signature, issuer, audience, expiry/not-before,
  algorithm allowlist, and subject mapping. Resolve tenant membership and
  mailbox permissions server-side. Never accept client tenant/user headers as
  identity.
- **Tests:** Valid/invalid signatures, wrong issuer/audience, expiry and
  not-before, key rotation/JWKS outage behavior, unknown/disabled users,
  tenant-membership isolation, and mailbox permission checks using provider
  fixtures.

### 3. Persist remaining control-plane domain and usage state

- **Dependencies:** Milestone 1; API keys and optional Keycloak bearer-token
  authentication are available for provisioned principals. Define usage
  window and accounting semantics before schema selection.
- **Files:** `smtp-api/internal/controlplane/handler.go`, new or extended
  PostgreSQL repository code, startup wiring, tests, OpenAPI, and
  `docs/phase-status.md`.
- **Database change:** The `domains` table already exists; use it for durable
  domain operations. No durable tenant usage counter/table is present in the
  inspected PostgreSQL migrations, so a migration is required after deciding
  whether counts derive from queue events, submissions, or another durable
  source.
- **Security:** Every operation remains tenant-scoped from the authenticated
  principal. DNS verification secrets must remain hashed and never appear in
  logs. Usage updates must be atomic under concurrent requests.
- **Tests:** Cross-tenant read/write denial, status transitions, duplicate
  domain conflicts, secret non-disclosure, and concurrent/period-boundary
  usage accounting with PostgreSQL.

### 4. Expose authenticated mailbox lifecycle and mutation APIs

- **Dependencies:** Milestones 1 and 2's identity policy decision; select
  stable endpoint/idempotency semantics and audit requirements.
- **Files:** Go handlers and PostgreSQL repository, OpenAPI, schema/migration
  only if the existing mailbox tables and outbox cannot represent the chosen
  contract, plus API and PostgreSQL integration tests. Rust workflows are
  reference behavior, not a Go dependency.
- **Database change:** Reuse existing mailbox, folder, permission, and
  workflow tables where semantics match. Add only forward migrations for
  missing idempotency/audit requirements.
- **Security:** Require mailbox `manage` for lifecycle/ACL writes, enforce
  tenant-consistent ownership, prevent privilege escalation, and audit
  membership/permission changes.
- **Tests:** Owner/admin/member matrix, tenant isolation, concurrent lifecycle
  changes, idempotency, rollback, and outbox consistency.

### 5. Wire inbound/outbound delivery to durable mailbox workflows

- **Dependencies:** Stable message append/deduplication behavior, mailbox
  lifecycle APIs, event ownership, and a durable worker/deployment choice.
- **Files:** `smtp-session/src/inbound.rs`, queue/delivery service boundaries,
  Go API submission path, `smtp-core/src/mailbox_store.rs`, and durable
  outbox consumer/worker code.
- **Database change:** Reuse existing inbox, sent-message, delivery projection,
  and outbox schema where sufficient; add a migration only for a documented
  event/idempotency gap.
- **Security:** Validate recipients against active tenant mailboxes, preserve
  tenant boundaries, protect raw content and attachment bytes, and make
  retries idempotent.
- **Tests:** End-to-end accepted/rejected inbound routing, outbound sent-copy
  and recipient state, retry/duplicate delivery, DSN handling, transaction
  failure, and recovery after worker restart.

### 6. Establish production capacity, SLO, backup, and recovery evidence

- **Dependencies:** Stable end-to-end flows and a selected production
  deployment/storage topology.
- **Files:** Operational runbooks and `docs/validation/`, load/recovery
  harnesses, metrics/alerts, deployment configuration, and storage adapters.
- **Database/storage change:** Object storage, partitioning, retention, or
  backup changes require measured workload and restore results; no provider or
  capacity numbers are assumed in this plan.
- **Security:** Key management, TLS, least privilege, retention/legal hold,
  deletion, and recovery access must be verified in the target environment.
- **Tests/evidence:** Load and soak tests against declared SLOs, failover,
  restore and point-in-time recovery drills, quota/retention correctness,
  security review, and monitored operational runbooks.

No milestone in this plan alone establishes production readiness. OIDC
integration is intentionally blocked until the identity provider contract is
approved.
