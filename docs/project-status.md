# Project status

## Current level

The repository has been advanced from its development/validation foundation into a secure, reliable, multi-tenant Email Cloud Platform across Milestones A through O.

- **Milestone A (Security & Identity):** Completed and verified. Features email/password authentication (Argon2id), single-use expiring verification links, Google & GitHub OAuth identity lookup, generic OIDC provider integration with PKCE and state/nonce one-time verification, strict HS256 JWT allowlists with kid rotation, opaque persistent browser sessions with refresh token family rotation, TOTP MFA with backup recovery codes, distributed PostgreSQL-backed rate limiting, encrypted transactional notification outbox, and comprehensive audit events.
- **Milestone B (Control Plane & Multi-Tenancy):** Completed and verified. PostgreSQL authoritative state for users, tenants, memberships, roles, domain verification, API keys, tenant plans, monthly usage metering (`usage_postgres.go`), durable audit records, and SMTP credentials. In-memory storage has been removed from production paths.
- **Milestone C (SMTP Protocol & Ingress):** Completed and verified. Explicit profiles for port 25 (inbound MX), 587 (authenticated submission STARTTLS), and 465 (SMTPS), TLS state reset, SASL PLAIN with tenant-scoped authentication, connection limits, strict command parsing, size negotiation, and trusted `Received` header generation.
- **Milestone D (Durable Queue & Outbound Delivery):** Completed and verified. Durable PostgreSQL queue with recipient-level tracking, domain concurrency leases, lease fencing, exponential retry backoff, temp/permanent error classification, remote MX discovery, null MX rejection, DNS safety, WebPKI TLS verification, destination IP throttling, operational kill switch (`SMTP_DELIVERY_KILL_SWITCH`), and tenant suspension enforcement.
- **Milestone E (Inbound Mail & Deliverability):** Completed and verified. Verified domain routing, tenant-isolated multi-recipient fanout, atomic acceptance, trusted Received + Authentication-Results, ClamAV/SpamAssassin fail-closed scanning, SPF/DKIM verification, and DMARC aggregate reporting.
- **Milestone F (DKIM, Domain Security, DNS):** Completed and verified. Ed25519 key generation (`dkim-keygen`), filesystem permissions isolation (0600), DNS TXT guidance, DNSSEC-aware key verification (`dkim-keyctl activate`), selector rotation and retirement (`dkim-keyctl retire`), and multi-domain signing. MTA-STS, TLS-RPT, and DANE RFC profiles are documented and external-DNS ready.
- **Milestone G & H (Mailbox, IMAP, & POP3 Decision):** Completed and verified. The `mailbox-service` provides mailbox CRUD, permissions, folder hierarchies, append, flags, search, threading, retention, and legal hold. `smtp-imap` provides a bounded IMAP4rev1/IMAP4rev2 protocol engine. POP3 is explicitly excluded from the initial production release and documented across repository manifests; no POP3 listener is deployed.
- **Milestone I (Object Storage):** Bounded message payloads (up to 10MB) are stored durably within PostgreSQL ACID transactions. External cloud S3/GCS buckets are classified as `BLOCKED BY EXTERNAL PREREQUISITE` pending cloud deployment target provisioning.
- **Milestone J (Bounces, Suppressions, Events):** Completed and verified. Durable bounce and complaint ingestion, suppression enforcement on enqueue, delivery event history, durable webhook subscriptions, SHA-256 HMAC payload signatures, and retry worker with dead-letter replay.
- **Milestone K (API & Dashboard):** Completed and verified. Versioned REST API with synchronized OpenAPI 3.0.3 specification (`docs/api/openapi.yaml`) and production Next.js 16 control plane dashboard.
- **Milestones L, M, N, O (Lifecycle, Observability, Security, Scale):** Completed and verified. Retention policies, OpenTelemetry tracing and structured logs, health probes (`/health/ready`), `cargo audit` passing with `.cargo/audit.toml` exemption for non-vulnerable upstream advisory, `govulncheck` passing with 0 vulnerabilities, least-privilege CI workflows, and concurrency/lease testing passing.

The authoritative milestone completion matrix is maintained in [smtp cloud production completion plan](audits/smtp-cloud-production-completion-plan.md).

## Completed capabilities

- Production-grade security and authentication: Argon2id password hashing, Google/GitHub OAuth, OIDC, TOTP MFA, refresh token rotation, and encrypted outbox.
- Authoritative PostgreSQL control plane: Tenants, users, memberships, API keys, domains, usage quotas, audit logging, and SMTP credentials.
- Multi-profile SMTP ingress: Ports 25, 587, and 465 with STARTTLS, SASL PLAIN, strict line length and command syntax enforcement.
- Outbound delivery engine: Bounded concurrency, domain leasing, MX discovery, Null MX handling, WebPKI TLS verification, operational kill switches, and suspended tenant exclusion.
- Inbound deliverability: SPF, DKIM, and DMARC verification, ClamAV/SpamAssassin integration, and Authentication-Results header emission.
- DKIM management tooling: `dkim-keygen` and DNSSEC-aware `dkim-keyctl` CLI utilities for key generation, verification, and rotation.
- Mailbox service: Complete mailbox lifecycle, shared permissions, folder management, message threading, retention, and legal hold.
- Webhooks and feedback loop: Signed webhooks with HMAC, delivery worker retries, dead-letter storage, and manual replay.
- Fully synchronized OpenAPI specification and Next.js 16 control plane dashboard.
- Verified test suites: `make check`, `cargo audit`, `govulncheck`, `pnpm test`, and `validate-migrations.sh` pass cleanly.
