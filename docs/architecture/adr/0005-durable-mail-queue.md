# ADR 0005: PostgreSQL durable mail queue

- Status: Accepted
- Date: 2026-10-28

## Context

The SMTP prototype previously retained accepted messages only in memory. A
durable queue must survive process failure while remaining local and testable.

## Decision

Use PostgreSQL tables for bounded message/envelope data, queue state, idempotency
records, leases, and attempt history. Workers claim due rows with
`FOR UPDATE SKIP LOCKED`, commit a unique lease token, process outside the
transaction, and require that token for completion.

## Alternatives considered

Kafka, RabbitMQ, Redis, and a separate object store add infrastructure that is
out of scope. An in-process queue does not preserve committed messages across
restarts.

## Consequences

Enqueue is atomic and inspectable locally, and multiple workers can claim work
concurrently. The worker provides at-least-once processing attempts only. The
test processor never contacts recipient servers, and this is not a production
delivery service.
