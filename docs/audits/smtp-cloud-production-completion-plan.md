# SMTP cloud production completion plan

**Audit date:** 2026-11-01
**Branch:** `smtp-cloud-production-completion`
**Method:** executable entry points, PostgreSQL migrations, tests, and current
configuration were inspected. Existing status documents were treated only as
leads; they do not establish completion.

## Completion register

| Milestone | Current evidence | State | Next dependency / acceptance evidence |
|---|---|---|---|
| A — identity | Password lifecycle, PostgreSQL sessions, DB limiting and encrypted mail outbox exist in `smtp-api/internal/controlplane`; OAuth flow state is cookie-only before this change. MFA/WebAuthn and refresh-token families are absent. | IMPLEMENTING | Persist one-time OAuth/OIDC flows (migration 000037), then MFA/passkeys and session replay tests. |
| B — control plane | Tenant/domain/key/credential repositories are PostgreSQL-backed; selected usage and product controls remain incomplete. | IMPLEMENTING | Remove remaining request-path local state; add tenant repository integration coverage. |
| C — SMTP ingress | Bounded ESMTP, TLS, AUTH and listener profiles exist in `smtp-session`; unsupported extensions are unadvertised. | TESTED (bounded profile) | Slow-client/fuzz/interoperability matrix and profile operations evidence. |
| D — outbound | Recipient queue, fencing/renewal and real SMTP worker exist. Fairness, IP throttles, circuits and external peer evidence are incomplete. | IMPLEMENTING | Implement controls and disposable-Postgres/fake-MX acceptance tests. |
| E — inbound | PostgreSQL routing/fanout/scanning/SPF-DKIM-DMARC adapters exist behind opt-in config. Recovery and bounded production operations are incomplete. | IMPLEMENTING | Durable outbox consumer/replay and tenant-isolation integration evidence. |
| F — DKIM/DNS | Projected-key signing and inbound verification exist. Managed keys, rotation and MTA-STS/TLS-RPT/DANE do not. | NOT STARTED | Secret-backed managed key lifecycle, migrations, RFC fixtures. |
| G — mailbox/IMAP | PostgreSQL mailbox service and bounded IMAP profile exist. Full protocol and lifecycle conformance remains incomplete. | IMPLEMENTING | Real auth integration and independent-client tests. |
| H — POP3 | Fixture crate only; explicitly excluded from release configuration. | VERIFIED (excluded) | Keep all POP3 listeners/UI claims absent until a real service is funded. |
| I — object storage | Inline PostgreSQL bodies only; no object-store adapter. | NOT STARTED | S3-compatible streaming storage, scan/quarantine, reconciliation tests. |
| J — bounces/events | Recipient suppression storage, transactional enqueue enforcement, tenant-scoped management APIs, and a durable delivery-event read model now exist. Bounce/DSN ingestion and durable webhook delivery remain incomplete. | IMPLEMENTING | Add race-safe bounce ingestion, signed durable webhooks, fake receiver tests, and event replay. |
| K — API/dashboard | Versioned OpenAPI and many tenant APIs exist; contract/dashboard completeness is not verified. | IMPLEMENTING | API inventory against OpenAPI and remove unsupported UI controls. |
| L — lifecycle/search | Search primitives, archive retention metadata, legal-hold fields, and a retention worker now exist; export, tombstones, and cross-store deletion propagation remain incomplete. | IMPLEMENTING | Add audited tombstones/export and reconciliation evidence. |
| M — operations | Local metrics/logging and preflight exist. Hosting-specific telemetry, dashboards and DR evidence are external prerequisites. | BLOCKED BY EXTERNAL PREREQUISITE | Select hosting/telemetry/secrets environment; then validate deployment and recovery. |
| N — supply chain | CI baseline exists; RustSec findings and provenance/image controls remain. | IMPLEMENTING | Upgrade/replace vulnerable paths; SBOM, signing and scans in CI. |
| O — scale | No reproducible production-scale results were found. | BLOCKED BY EXTERNAL PREREQUISITE | Provision isolated load environment and record raw workload/results. |

## Implemented in this pass

The first A dependency is a durable OAuth/OIDC authorization-flow record:
provider-bound state, nonce and PKCE-verifier hashes, optional tenant choice,
five-minute expiry, atomic one-time consumption, and a cleanup query. Callback
cookies remain browser binding material, but are no longer the only replay
defence. The flow is deliberately fail-closed when PostgreSQL is unavailable.

## Verification order

1. Apply migrations to a disposable PostgreSQL database and run control-plane
   integration tests for expiry, replay, provider mismatch and tenant binding.
2. Run Go formatting, unit/race/vet/build checks; then Rust workspace and
   dashboard checks without altering unrelated working-tree changes.
3. Continue from the first non-verified register entry. External deployment,
   DNS, scanner, secrets-manager and load-environment claims stay blocked until
   independently exercised; no local diagnostic changes that status.
