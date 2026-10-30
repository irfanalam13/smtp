# Phase 109 — Queue Clustering and Coordination

Status: **Durable PostgreSQL queue primitives; multi-worker and leader-election behavior not production-validated**

## Repository baseline

The project has PostgreSQL-backed queue leases, acknowledgements, retry
primitives, and a deterministic local worker. There is no production broker,
clustered general event bus, leader-election service, or high-volume throughput
evidence. The queue's critical invariant is durable acceptance and safe
recovery, not the presence of Kafka/RabbitMQ/NATS by itself.

## Recommended design

Measure queue throughput, latency, fanout, retention, ordering, and replay
needs before selecting a broker. Preserve transactional outbox/inbox or
equivalent idempotency at service boundaries. Use bounded leases, fencing
tokens, visibility timeouts, dead-letter policy, poison-message quarantine,
and backpressure. Leader election is required only for genuinely singleton
work; use a well-understood consensus/lease facility with fencing so a paused
old leader cannot continue acting after lease loss. Never make SMTP message
acceptance depend on an analytics or notification consumer.

## Remaining work and why

Establish queue SLO and workload; implement production processors, retry/DLQ
and replay controls, consumer idempotency, schema evolution, observability,
and operational recovery. Add broker/leader election only after a measured
requirement and independent failure tests. This remains open because current
workers are deterministic development processors and the event consumers are
not deployed.

## Acceptance criteria

- Worker crash, duplicate delivery, expired lease, delayed task, and poison
  message tests show no lost accepted work or unbounded retry.
- Stale leaders are fenced; only one valid actor performs singleton work.
- Backpressure and destination isolation prevent queue growth from exhausting
  database or SMTP capacity.
- Outbox/event replay is idempotent and schema-compatible across versions.
- Throughput/latency/lag objectives are tested under node and dependency loss.

## Research references

- [Durable mail queue architecture](../architecture/durable-mail-queue.md)
- [Phase 82 Event Bus and Messaging](phase-82-event-bus-messaging.md)
- [Phase 05 durable mail queue](../architecture/adr/0005-durable-mail-queue.md)
