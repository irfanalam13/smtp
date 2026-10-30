# Phase 82 — Event Bus and Messaging

Status: **PostgreSQL queue/outbox foundation exists; general event transport and operational consumers pending**

## Repository baseline

The architecture selects PostgreSQL queue/outbox as the initial durable
mechanism, with idempotent and replayable consumers. The repository has a
mail-delivery queue and a mailbox-specific event outbox, but no Kafka,
RabbitMQ, or NATS deployment, general event schema registry, or complete
consumer fleet.

## Recommended approach

Start with transactional outboxes and bounded PostgreSQL polling for the
current workload. Define versioned event envelopes with stable IDs, source,
type, occurrence time, tenant scope where appropriate, correlation, and
schema version; use CloudEvents as an interoperability reference rather than
forcing every internal table to match a wire profile. Consumers must be
idempotent, replayable, and tolerant of duplicate and out-of-order delivery.
Partition/order only where a business invariant requires it.

Evaluate Kafka, RabbitMQ, or NATS only against measured throughput, retention,
fan-out, ordering, replay, latency, tenant isolation, operational ownership,
and recovery requirements. A broker does not remove the need for transactional
outboxes, schema evolution, authorization, poison-event handling, or consumer
idempotency. Never synchronously publish external webhooks, analytics, or
billing from the SMTP acceptance transaction.

## Remaining work and why

Inventory event producers/consumers; standardize event schemas and evolution;
finish the webhook/outbox worker; add idempotent consumers, replay,
dead-lettering, lag/retention dashboards, access controls, and recovery
procedures. Benchmark PostgreSQL polling under realistic tenant skew before
selecting a dedicated broker and documenting migration/dual-publish behavior.

This remains open because the repository has durable queue/outbox components
but not a general event platform. Selecting a broker before workload and
operations are known adds stateful infrastructure without proving that it
solves a current bottleneck.

## Acceptance criteria

- Domain writes and corresponding outbox events commit atomically.
- Event schemas are versioned, bounded, privacy-reviewed, and backward
  compatible or have explicit migrations.
- Consumers handle duplicates, retries, poison events, and replay with
  measurable lag and no silent loss.
- Tenant-scoped event access and retention are tested.
- Broker selection is justified by repeatable workload and operational
  evidence, with restore and failover exercises.

## Research references

- [CloudEvents 1.0.2](https://github.com/cloudevents/spec/blob/v1.0.2/cloudevents/spec.md)
- [Transactional outbox ADR](../architecture/adr/phase-01/0013-event-architecture.md)
- [Phase 81 Webhook System](phase-81-webhook-system.md)
