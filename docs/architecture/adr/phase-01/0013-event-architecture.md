# ADR-0013: Transactional outbox and durable events

- Status: Proposed
- Date: 2026-10-28

## Context

Webhooks, analytics, usage, audit, and suppression must not block acceptance or delivery, but cannot lose source events silently.

## Decision

Write an outbox row with the authoritative transaction, publish it asynchronously to a durable event stream, and make all consumers idempotent and replayable. PostgreSQL-backed queue/outbox is the initial implementation; a dedicated broker is introduced only after throughput/retention evidence.

## Consequences

Consumers lag independently and source state remains authoritative. Outbox growth, poison events, schema compatibility, and replay access require operations and retention controls.
