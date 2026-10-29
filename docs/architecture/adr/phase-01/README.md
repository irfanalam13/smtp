# Phase 01 ADR index

The repository already contains accepted implementation ADRs [`../`](../) for the monorepo, service boundaries, migrations, local development, durable queue, control-plane isolation, and API-key handling. The records below capture the broader Phase 01 target architecture without invalidating those records.

| ADR                                   | Decision                                                     | Status   |
| ------------------------------------- | ------------------------------------------------------------ | -------- |
| [0010](0010-architectural-style.md)   | Small independently scalable services with modular internals | Proposed |
| [0011](0011-control-data-plane.md)    | Control/data-plane separation                                | Proposed |
| [0012](0012-message-acceptance.md)    | Durable acceptance and at-least-once delivery                | Proposed |
| [0013](0013-event-architecture.md)    | PostgreSQL outbox and durable events initially               | Proposed |
| [0014](0014-tenant-isolation.md)      | Tenant-scoped ownership and defense in depth                 | Proposed |
| [0015](0015-initial-product-scope.md) | Transactional delivery first; mailbox later                  | Proposed |
| [0016](0016-open-source-license.md)   | Apache-2.0 direction pending governance/legal review         | Proposed |
