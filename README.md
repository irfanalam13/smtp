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
dashboard, SMTP ingress, and deterministic queue worker. Use
`./scripts/deploy-preflight.sh --production-env` to validate a production
environment and build release artifacts. These scripts prepare and run the
repository but do not deploy it to a cloud provider.

Message contents and credentials must never be written to logs.

Milestone 1 adds a loopback-only SMTP protocol prototype in
`smtp-session`. It supports a bounded test transaction but does not
deliver mail, persist messages, implement TLS or authentication, or provide a
public SMTP service. TLS, authenticated submission, abuse controls, durable
queues, and delivery are required follow-up work.

Milestone 2 adds a PostgreSQL-backed durable queue with idempotent enqueue,
lease-based claims, bounded retries, attempt history, and a deterministic test
processor. `queued` and `simulated_succeeded` are internal states; neither
means that a remote recipient received the message.

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
events, and tenant authorization are implemented. OIDC, SMTP AUTH
credentials, Go-to-queue message submission, and persisted delivery read
models remain deferred rather than being represented as fake functionality.

Milestone 5 adds an opt-in inbound reception path in
`smtp-session/src/inbound.rs`. It routes only to active mailboxes on
verified domains and stores the exact raw message plus routing metadata in one
PostgreSQL transaction before returning SMTP `250`. The mailbox API and
dashboard expose metadata only. The default listener remains loopback-only;
public MTA deployment, TLS certificates, raw-message streaming, retention
jobs, and production object storage require explicit future work.

Milestone 6 adds authenticated domain DNS diagnostics and readiness views for
SPF, DMARC, MX, and existing ownership verification. Checks are bounded,
cached, rate-limited, and advisory; they do not prove inbox placement. Usage
uses a conservative development-only one-hour quota because the Go control
plane remains in-memory. DKIM signing is not implemented and is not implied by
these checks.

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
