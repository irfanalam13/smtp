# Phase 171 — Distributed Message Queue

Status: **PostgreSQL queue primitives exist; no multi-region broker or worker fleet**

## Recommended design

Choose regional queue ownership and replication semantics before broker
selection. Preserve durable acceptance, idempotency, lease fencing, bounded
retries, destination rate limits, and dead-letter/replay controls. Avoid
active-active delivery of the same message unless a globally consistent
deduplication and recipient outcome model is proven. Isolate regional
backlogs and define controlled draining during failover.

## Remaining work and why

Measure queue workloads and select broker/replication model, ownership,
residency, and recovery. This remains open because there is no distributed
worker deployment or tested regional queue failover.

## Acceptance criteria

- Message acceptance remains tied to durable commit.
- Regional failover cannot double-claim or silently lose delivery work.
- Retry storms and destination quotas are controlled across regions.
- Replication lag, queue age, dead-letter, replay, and ownership are visible.
- Backlog recovery meets tested objectives without saturating dependencies.

## Research references

- [Durable mail queue architecture](../architecture/durable-mail-queue.md)
- [Phase 109 Queue Coordination](phase-109-queue-coordination-leader-election.md)
- [Phase 114 Asynchronous Processing](phase-114-asynchronous-processing-and-mail-queues.md)
