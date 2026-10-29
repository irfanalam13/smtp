# ADR-0017: Shared core infrastructure boundary

- Status: Proposed
- Date: 2026-10-28

## Context

The repository has multiple Rust services but no shared configuration,
secret-handling, context, event-envelope, logging, or shutdown contracts.
Creating many empty crates would add dependency and release overhead before
there are independent ownership boundaries.

## Decision

Evolve the existing `shared-types` crate (`smtp-core`) into the focused shared
core infrastructure crate. It owns typed configuration primitives, redacted
secret values and local providers, validated operation identifiers, event
envelopes and publisher contracts, logging initialization, and bounded shutdown
signals. It does not own service business logic, database schemas, a broker
adapter, authentication, or the observability backend.

## Consequences

Rust services share contracts without a new microservice or a dependency cycle.
The crate must keep its public API small and avoid becoming a general-purpose
utility dump. A module may become an independently released crate only when it
has distinct ownership, dependency, scaling, or security requirements.

## Deferred decisions

Phase 01 selected a PostgreSQL queue/outbox initially and did not select a
dedicated broker. Phase 04 therefore defines event contracts but does not
pretend to provide durable transport. Transactional outbox implementation
belongs with Phase 06 storage work.
