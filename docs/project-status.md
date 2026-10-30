# Project status

## Current level

Milestones 0–3 are implemented and validated as a local protocol, queue, and
outbound-delivery foundation. Milestone 4 adds a functional development
control plane and dashboard. Milestone 5 adds opt-in durable inbound mail.
Milestone 6 adds bounded DNS diagnostics, readiness, and development usage
views.

The repository is not yet a production hosted SMTP platform. The Go control
plane is still in-memory, development identity mode is supported, external
delivery is disabled by default, and no cloud deployment target is configured.

The complete phase-by-phase implementation register is in
[phase implementation status](phase-status.md). It records remaining code,
the reason each item is deferred, and the phase or prerequisite when it should
be applied. Phase completion for its documented scope must not be interpreted
as production readiness. Phase 10 validation evidence is collected in
[the validation reports](validation/README.md), with an explicit
release-readiness decision and risk register. The complete handoff analysis for
Phases 01–10 is in the [phase integration review](phase-integration.md).

## Completed capabilities

- Rust SMTP ingress with bounded protocol behavior and graceful shutdown.
- PostgreSQL durable queue with leases, retries, and attempt history.
- Controlled outbound SMTP client with destination safety checks and STARTTLS.
- Tenant, domain, API-key, role, scope, and audit foundations.
- Durable inbound mailbox/message persistence when explicitly configured.
- SPF, DMARC, MX, and ownership diagnostics with bounded caching and rate
  limiting.
- Local bootstrap, normal development runner, and deployment preflight
  scripts.
- Configuration, testing, operations, data-lifecycle, and documentation index
  guides.
- Rust, Go, dashboard, migration, and CI validation paths.

## Production blockers

- Replace the in-memory Go control-plane store with PostgreSQL repositories.
- Complete Keycloak OIDC user provisioning, browser session management, and
  MFA policy evidence.
- Add a real send API and connect it to the durable queue.
- Enforce sender-domain authorization at the enqueue boundary.
- Make quota accounting durable and distributed.
- Add persisted delivery read models, bounce handling, suppression, retention,
  and abuse controls.
- Implement DKIM signing, key storage, rotation, and verification tests.
- Choose and configure a production runtime, secrets manager, observability
  stack, and deployment target.
- Run PostgreSQL-backed integration tests in CI and perform load/failure tests.
- Remediate the RustSec findings reported by `cargo audit`, including the
  vulnerable `idna` and `rsa` dependency paths and the unmaintained
  `trust-dns-proto` dependency.
- Verify repository secret scanning, branch protection, release controls, and
  production telemetry in the target GitHub/deployment environment.

## Script lifecycle

| Use case | Command |
| --- | --- |
| First checkout or dependency/schema refresh | `./scripts/bootstrap-dev.sh` |
| Normal local development | `./scripts/run-dev.sh` |
| Normal start without queue worker | `./scripts/run-dev.sh --no-worker` |
| Stop local PostgreSQL | `make dev-down` |
| Production configuration/build preflight | `./scripts/deploy-preflight.sh --production-env` |
