# Phase 114 — Asynchronous Processing and Mail Queues

Status: **Durable queue primitives exist; distributed worker capacity and operations pending**

## Goal and scope

Scale SMTP acceptance and delivery independently using durable asynchronous
processing. Preserve the SMTP durability boundary, queue lease safety,
idempotent delivery behavior, backpressure, retry policy, and dead-letter
operations as worker concurrency increases.

## Repository baseline

The repository includes PostgreSQL mail queue and lease/retry primitives,
but does not have a production clustered queue, broker, delivery fleet, or
validated horizontal worker deployment. The queue's existence does not prove
at-least-once delivery safety or external delivery success at scale.

## Recommended design

Keep SMTP acceptance coupled to a durable commit before returning success.
Workers should claim bounded batches, use lease fencing/expiry, perform
idempotent state transitions, and apply exponential backoff with jitter and
destination-aware throttling. Bound per-tenant and per-domain concurrency,
message size, retry volume, and in-flight work. Separate transient, permanent,
policy, and operator-review outcomes. Queue age and oldest-item time are
first-class health signals.

Only select a broker after comparing ordering, retention, replay, operational
ownership, and transactional handoff requirements against the existing
PostgreSQL queue. Avoid a dual-write design without an outbox or equivalent
atomic handoff.

## Remaining work and why

Run representative worker load and failure tests, including process death
during SMTP delivery, duplicate claims, remote throttling, and backlog recovery.
Production queue expansion depends on Phases 82, 106, 109, and 110 and cannot
be declared complete from the local queue implementation.

## Acceptance criteria

- SMTP accepts only after durable queue commitment and survives a crash/restart.
- Lease expiration, worker duplication, shutdown, retries, and replay cannot
  silently lose or corrupt queue state.
- Backpressure and per-destination limits prevent overload and retry storms.
- Queue age, depth, retries, terminal failures, and throughput are measured.
- A backlog can be drained within a tested recovery objective without harming
  interactive API or mailbox traffic.

## Research references

- [Durable mail queue architecture](../architecture/durable-mail-queue.md)
- [Phase 16 Mail Queue Engine](phase-16-mail-queue-engine.md)
- [Phase 82 Event Bus and Messaging](phase-82-event-bus-messaging.md)
- [Phase 109 Queue Coordination and Leader Election](phase-109-queue-coordination-leader-election.md)
