# SMTP Platform

This repository contains the SMTP cloud platform foundation: Rust protocol
workspaces, a Go control plane, a Next.js dashboard, and PostgreSQL migrations.

See [development setup](docs/development/setup.md) for prerequisites and
commands. The [documentation index](docs/README.md) explains the architecture,
security model, operations, and current production limitations.

The [phase implementation status](docs/phase-status.md) records remaining
implementation for every phase, why it is deferred, and when it should be
applied.

The [Phase 01–10 integration review](docs/phase-integration.md) maps each
phase handoff to the connected platform components and records the completion
boundary through Phase 10. The foundation is complete for its documented
scope and validated with limitations; it is not yet a production-ready hosted
SMTP platform.

For the Phase 03 development workflow, start with the
[development guide](docs/development/README.md), run `make doctor`, and use
`make help` for the supported command interface. Local infrastructure details
are in the [local infrastructure guide](docs/development/local-infrastructure.md).

For a new checkout, run `./scripts/bootstrap-dev.sh` once. For normal
development, run `./scripts/run-dev.sh`; it starts PostgreSQL, the API,
dashboard, and SMTP ingress. Outbound delivery is opt-in; use
`./scripts/run-dev.sh --worker` with an explicit development-only SMTP relay
or a direct-delivery allowlist. Local Mailpit setup and the API submission
workflow are documented in the [development guide](docs/development/setup.md).
Use `./scripts/deploy-preflight.sh --production-env` to validate a production
environment and build release artifacts. These scripts prepare and run the
repository but do not deploy it to a cloud provider.

Message contents and credentials must never be written to logs.

Milestone 1 implements TLS-secured SMTP submission in `smtp-session`,
including STARTTLS and optional implicit TLS, TLS-only AUTH PLAIN,
tenant-scoped PostgreSQL credential verification and lifecycle APIs,
verified-domain sender authorization, shared authentication throttling,
certificate reload, bounded connection handling, and graceful draining. The
listener remains loopback-only by default; public authenticated submission
requires TLS, PostgreSQL, and shared throttling configuration. The M01
acceptance suite covers a real Python `smtplib` submission, credential
isolation and lifecycle, STARTTLS failure/reset behavior, connection limits,
timeouts, and shutdown. Queue acceptance is not outbound delivery, and
provider-specific deployment configuration remains an operational concern.

Milestone 2 adds a PostgreSQL-backed durable queue with idempotent enqueue,
lease-based claims, bounded retries, attempt history, and a deterministic test
processor used by queue tests only. `queued` and `simulated_succeeded` are
internal states; neither means that a remote recipient received the message.

Milestone 3 adds controlled outbound SMTP delivery in
`smtp-delivery`. It is disabled by default, uses MX resolution and
destination safety filtering, and supports bounded opportunistic or required
STARTTLS. A remote SMTP acceptance response is distinct from inbox delivery;
the platform does not claim mailbox placement.

Milestone 4 adds the initial tenant-scoped control plane. The versioned API
contract is in [`docs/api/openapi.yaml`](docs/api/openapi.yaml). Local
development uses an explicitly enabled `CONTROL_PLANE_DEV_MODE` principal
adapter; it is not a production authentication system. Domain registration,
pending verification state, scoped one-time API keys, revocation, audit
events, tenant authorization, SMTP credential management, OIDC API-token
verification, and generic OIDC browser sessions built on `smtp-auth` are
implemented for provisioned identities. Password account lifecycle,
provisioning, MFA/passkeys, GitHub OAuth identity lookup, distributed auth
rate limits, and production provider operations remain incomplete. Persisted
delivery read models remain deferred. The dashboard now uses a same-origin API
proxy and can submit plain-text messages through a scoped HTTP endpoint into
the durable PostgreSQL queue. Queue acceptance is not delivery. A separately
configured real `mail-delivery` worker handles external SMTP; it remains
disabled by default and M05 is not production-ready.

Milestone 5 adds an opt-in inbound reception path in
`smtp-session/src/inbound/mod.rs`. It routes active primary and alias addresses on
verified domains and atomically fans out across mailboxes belonging to one
tenant. The SMTP executable wires PostgreSQL acceptance only when
`SMTP_INBOUND_ENABLED=true`; the default listener remains loopback-only. It
stores exact raw bytes and routing metadata before returning SMTP `250`. The
mailbox API and dashboard expose metadata only. Public inbound requires
configured ClamAV scanning and shared PostgreSQL rate limits. SPF/DKIM/DMARC
enforcement, the outbox consumer, retention jobs, and production object storage
remain future work.

Milestone 6 adds authenticated domain DNS diagnostics and readiness views for
SPF, DMARC, MX, and existing ownership verification. Checks are bounded,
cached, rate-limited, and advisory; they do not prove inbox placement. Usage
uses a conservative development-only one-hour quota because the Go control
plane remains in-memory. DKIM signing is not implemented and is not implied by
these checks.

Milestone 8 connects mailbox administration and message operations to a
PostgreSQL-backed Rust mailbox service behind the authenticated Go API. It
includes mailbox-scoped permissions, transactional quota/UID allocation,
idempotent append, folders, aliases, labels, and audited content reads. The
message store remains inline PostgreSQL and the service is not a production
hosted-mailbox or IMAP offering.

The current implementation status and deferred work are tracked in
[current status](docs/project-status.md). The repository is a development and
validation foundation, not yet a production-ready hosted SMTP service.

Phase 10 validation evidence is available in the
[validation reports](docs/validation/README.md), including the requirements
matrix, recovery evidence, performance baseline, risk register, and
release-readiness assessment.

CI/CD workflows, security automation, and release instructions are documented
in the [DevOps guide](docs/devops/).

# smtp
