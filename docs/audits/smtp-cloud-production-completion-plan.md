# SMTP Cloud production completion plan

Audit date: 2026-11-01
Branch: `smtp-cloud-production-completion`

This tracker records only repository evidence. `VERIFIED` requires an
implementation, executable validation, and an acceptance result. External
credentials, DNS, production hosting, secret-management, and disposable
PostgreSQL infrastructure are recorded as blockers rather than treated as
complete.

| Milestone | Current evidence | Status | Next implementation or evidence |
|---|---|---|---|
| A Security and identity | Password lifecycle, OAuth hardening, encrypted auth-email outbox, and shared API throttling exist. MFA, passkeys, durable refresh-token rotation, and provider identity provisioning are absent. | IMPLEMENTING | Add only against the existing PostgreSQL identity model; verify with integration tests. |
| B Control plane and tenancy | Selected API/domain/mailbox paths use PostgreSQL; development identity, usage, and some audit paths remain process-local. | PARTIAL | Inventory each request path and move production-critical state to repositories. |
| C SMTP ingress | Bounded SMTP, STARTTLS/implicit TLS, AUTH PLAIN, inbound trace, and explicit listener profiles exist. PIPELINING, SMTPUTF8, DSN, CHUNKING, and BINARYMIME are correctly unadvertised. | TESTED WITH LIMITATIONS | Add role-specific disposable-peer tests; external protocol validation remains required. |
| D Queue and outbound delivery | Durable queue, recipient outcomes, lease renewal/fencing, MX safety, and real worker exist. Shared fairness, IP budgets, circuit breakers, MTA-STS, TLS-RPT, and DANE are absent. | IMPLEMENTING | Add durable/configured delivery controls without introducing process-local production state. |
| E Inbound and deliverability | Inbound persistence, scanner integration, SPF/DKIM/DMARC evaluation, fan-out, event projection, retries, and dead letters exist. Recovery and disposable-DB evidence remain incomplete. | TESTED WITH LIMITATIONS | Add integration/reconciliation evidence and explicit DNS/scanner health. |
| F DKIM and DNS security | File-projected Ed25519 signing and diagnostics exist; tenant key lifecycle, DNSSEC-aware verification, MTA-STS, TLS-RPT, and DANE are absent. | PARTIAL | Implement lifecycle abstractions only where a secret-manager boundary is available. |
| G Mailbox and IMAP | PostgreSQL mailbox API and bounded mailbox operations exist; storage is inline BYTEA and IMAP is not connected to authenticated mailbox data. | PARTIAL | Complete retention/object-storage boundary or keep hosted mailbox out of release scope. |
| H POP3 | `smtp-pop3` is a parser/session prototype without a network service or real authentication. | VERIFIED WITH EXCLUSION | Release docs now explicitly exclude POP3; no listener, configuration, or production claim exists. |
| I Object storage and attachments | Attachment/message abstractions are in-memory or inline PostgreSQL; no provider-neutral durable S3 adapter is wired. | BLOCKED BY EXTERNAL PREREQUISITE | Requires selected S3-compatible provider, credentials, encryption, and integration environment. |
| J Bounces, suppression, and events | Webhook signing helpers and DMARC reporting foundations exist. Recipient suppression is now durably checked inside the enqueue transaction; management APIs, bounce/DSN ingestion, and webhook retries remain absent. | IMPLEMENTING | Add authorized suppression management, bounce ingestion, event history, and durable webhook delivery. |
| K API and dashboard | Existing OpenAPI and dashboard cover current control-plane features; future/unsupported protocols must remain hidden. | PARTIAL | Synchronize only implemented backend capabilities and remove misleading controls. |
| L Search, retention, lifecycle | Search/index schemas and deletion primitives exist; retention, legal hold, export, tombstones, and cross-store reconciliation are incomplete. | PARTIAL | Implement against the selected durable storage boundary. |
| M Operations | Health endpoints, structured logs, and process-local Prometheus metrics exist; no deployed telemetry backend or production target is selected. | BLOCKED BY EXTERNAL PREREQUISITE | Requires hosting, collector, alert routing, secret manager, and backup target. |
| N Supply chain | `cargo audit` now reports only RSA/Marvin; quick-xml, idna, trust-dns, and rustls-pemfile findings are cleared or removed. RSA is pulled by `mail-auth` for standards-based verification and has no fixed upgrade. | BLOCKED BY RELEASE RISK | Keep RSA private-key operations disabled; obtain upstream fix or formally accept the verifier-only residual risk before production release. |
| O Scale and recovery | Unit and selected integration tests exist; representative load, chaos, restore, and external interoperability evidence is incomplete. | BLOCKED BY EXTERNAL PREREQUISITE | Requires disposable PostgreSQL, fake MX peers, storage, scanner, and target hardware. |

## Immediate implementation order

1. Remediate fixed Rust dependency advisories without changing protocol
   behavior.
2. Make POP3 scope explicit and remove any release-facing implication that it
   is a service.
3. Add durable delivery-control primitives only after confirming queue schema
   and worker claim boundaries.
4. Add bounce/suppression lifecycle and enforce suppression before enqueue.
5. Add integration fixtures and update evidence only for tests that actually
   run.

The migration validator was attempted on this branch but could not run because
the required isolated `DATABASE_URL` was not configured. No production
database was accessed.

## Explicit external blockers

The repository cannot independently provision production DNS, reverse DNS,
SMTP egress, cloud object storage, KMS/secret management, scanner services,
telemetry collectors, branch protection, or a production hosting target.
Those prerequisites must be supplied and validated before the corresponding
requirements can become `VERIFIED`.
