# Project status

## Current level

The repository is a local and integration foundation, not a production hosted
SMTP platform. The Go API has PostgreSQL-backed paths for API keys, selected
tenant/domain operations, durable submission enqueue/history, and mailbox
reads. Development identity and usage state remain process-local. The SMTP
ingress can enqueue to PostgreSQL. A separate real outbound SMTP worker is now
available and disabled by default. Per-recipient state is persisted, but M05
is not production-ready: tenant/domain TLS policy, throttling, circuit
breaking, observability and interoperability acceptance remain incomplete.
The inbound PostgreSQL acceptor is wired only when
`SMTP_INBOUND_ENABLED=true`; it is not part of the default SMTP runtime.
M08 now wires authenticated Go API calls through a signed internal principal
to the Rust mailbox service for mailbox administration and message/folder
operations. Mailbox writes enforce user membership, mailbox grants, transactional
quota, and append idempotency. This remains a partial mailbox platform: bytes
are inline PostgreSQL `BYTEA`; inbound accepted/quarantined events are projected
to INBOX/Junk with transactional acknowledgment, but scheduled retention, object
storage, outbound sent-copy/drafts, and deployment-grade workload identity
remain incomplete. Expired append-idempotency records are also cleaned up by a
bounded scheduled service task.

The current checkout's evidence-based service map, gap register, and baseline
are in the [M00 backend platform audit](backend-platform-audit.md). Historical
phase completion records apply only to their documented scopes and do not
imply end-to-end product readiness.

The complete phase-by-phase implementation register is in
[phase implementation status](phase-status.md). It records remaining code,
the reason each item is deferred, and the phase or prerequisite when it should
be applied. Phase completion for its documented scope must not be interpreted
as production readiness. Phase 10 validation evidence is collected in
[the validation reports](validation/README.md), with an explicit
release-readiness decision and risk register. The complete handoff analysis for
Phases 01–10 is in the [phase integration review](phase-integration.md).

## Completed capabilities

- Bounded SMTP ingress with optional STARTTLS/implicit TLS, TLS-gated
  PostgreSQL-backed AUTH PLAIN, shared database-backed throttling, automatic
  certificate reload from mounted files, and graceful active-session draining.
  Tenant-scoped credential lifecycle endpoints return generated secrets once
  and audit changes in PostgreSQL.
- PostgreSQL queue storage with idempotency, leases, retries, attempt history,
  per-recipient delivery records, and domain-partitioned claims.
- A separate real outbound SMTP worker connected to canonical-message
  validation and per-recipient results; delivery remains disabled by default.
- Tenant, domain, API-key, role, scope, and audit foundations.
- M08 mailbox data-plane service with signed Go-to-Rust requests, mailbox and
  alias/permission administration, folder and message mutations, quota
  accounting, audited content reads, idempotent append, inbound INBOX/Junk
  projection, and scheduled append-idempotency cleanup.
- Opt-in SMTP inbound acceptance wired to PostgreSQL. It atomically fans out
  across active mailboxes belonging to one tenant and rejects cross-tenant
  batches; it remains a bounded development/integration path.
- Inbound SPF, DKIM, and current-spec DMARC evaluation wired through a
  standards-focused Rust library. DMARC reject/quarantine policy results are
  enforced and summarized results are persisted with raw evidence.
- Outbound Ed25519 DKIM signing uses secret-manager projected PKCS#8 keys;
  real external delivery requires a signing key directory.
- SPF, DMARC, MX, and ownership diagnostics with bounded caching and rate
  limiting.
- Local bootstrap, normal development runner, and deployment preflight
  scripts.
- Configuration, testing, operations, data-lifecycle, and documentation index
  guides.
- Rust, Go, dashboard, migration, and CI validation paths.

## Production blockers

- Complete M05: tenant/domain TLS policy, destination IP throttling, circuit
  breakers, operational metrics/alerts, external SMTP interoperability testing,
  and production rollout controls.
- Complete M07 tenant-managed DKIM controls and conformance evidence; complete
  inbound operational recovery, mailbox retention, and object storage before
  enabling a full hosted-mailbox product. The inbound event projection worker
  is implemented; aggregate reporting and ingestion now have a bounded worker
  pipeline.
  Public inbound mode requires ClamAV, SpamAssassin, DNS access, and shared
  PostgreSQL per-IP/per-domain rate limits.
- Complete PostgreSQL-backed control-plane coverage; move development identity
  and usage/quota state out of process-local storage before production use.
- Complete the identity lifecycle beyond self-service email/password
  registration, verification, login, and recovery: OIDC user provisioning,
  MFA/passkeys, GitHub OAuth identity lookup, distributed authentication
  rate limiting, and production provider operations remain. Generic OIDC login
  and revocable browser sessions still require pre-provisioned identities.
- Connect the durable mailbox operations to a real authenticated IMAP service,
  or explicitly keep IMAP/POP3 out of product scope.
- Complete M08 with message-retention/deletion workflows,
  lifecycle/permission integration and reconciliation evidence; complete
  outbound sent-copy/drafts projection. Provide object storage and production
  workload identity before enabling hosted-mailbox traffic.
- Add durable object storage and scanning integration for raw messages and
  attachments.
- Add persisted delivery read models, bounce handling, suppression, retention,
  and abuse controls.
- Complete tenant-managed DKIM key storage, rotation APIs, and interoperability
  verification beyond the current file-projected Ed25519 signer.
- Choose and configure a production runtime, secrets manager, observability
  stack, and deployment target.
- Keep PostgreSQL-backed integration tests enabled in CI and perform
  production-environment load/failure tests.
- Configure and validate deployment-managed secret mounts/environment
  injection in the selected production environment. Direct cloud-provider
  secret APIs and OCSP policy are not implemented; expand SMTP client
  interoperability coverage beyond Python `smtplib`.
- Remediate the RustSec findings reported by `cargo audit`, including the
  vulnerable `idna` and `rsa` dependency paths and the unmaintained
  `trust-dns-proto` dependency.
- Verify repository secret scanning, branch protection, release controls, and
  production telemetry in the target GitHub/deployment environment.

## Script lifecycle

| Use case                                    | Command                                          |
| ------------------------------------------- | ------------------------------------------------ |
| First checkout or dependency/schema refresh | `./scripts/bootstrap-dev.sh`                     |
| Normal local development                    | `./scripts/run-dev.sh`                           |
| Start the real outbound worker              | `./scripts/run-dev.sh --worker` (explicit opt-in) |
| Stop local PostgreSQL                       | `make dev-down`                                  |
| Production configuration/build preflight    | `./scripts/deploy-preflight.sh --production-env` |
