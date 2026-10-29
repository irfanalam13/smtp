# ADR-0601: PostgreSQL storage boundary

- Status: Accepted
- Date: 2026-10-28

## Decision

PostgreSQL remains authoritative for all currently implemented durable state:
queue messages and leases, control-plane metadata, and opt-in inbound
messages. Existing migrations are immutable and forward-only. The shared core
exposes validated pool configuration and readiness, while domain crates retain
ownership of repositories and transactions.

## Consequences

The platform has one local durable dependency and avoids premature distributed
storage. Queue enqueue and inbound persistence provide database atomicity.
Cross-system event publication and large-object storage remain explicit future
boundaries.
