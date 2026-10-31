# Architecture decision records

The original implementation ADRs in this directory document the monorepo,
service boundaries, migrations, local development, durable queue, control-plane
isolation, and API-key handling.

The inbound routing policy is recorded in
[ADR 0008](0008-inbound-fanout-policy.md): fan-out is atomic and tenant-scoped.

The mailbox service ownership and Go-to-Rust principal boundary are recorded in
[ADR 0009](0009-mailbox-service-boundary.md).

The broader Phase 01 target architecture is documented in the
[Phase 01 ADR index](phase-01/).
