# Phase 16 — Mail Queue Engine

Status: **Durable queue foundation implemented; production policy hardening
continues**

## Contract

The queue is the responsibility-transfer boundary. A message is accepted only
after the envelope, body, recipients, and idempotency record are committed in
one PostgreSQL transaction.

Implemented in `smtp-queue`:

- durable message and recipient storage;
- scoped idempotency with payload-conflict detection;
- `FOR UPDATE SKIP LOCKED` claiming;
- owner/token leases and expired-lease recovery;
- attempt history and bounded diagnostics;
- temporary/internal retry classification;
- exponential backoff with jitter and delayed `next_attempt_at`;
- terminal state after the configured attempt budget;
- worker polling with cancellation.

The Phase 16 migration also aligns database constraints with SMTP’s null
reverse-path (`MAIL FROM:<>`) by allowing an empty envelope sender.

## Production requirements still pending

Priority classes, tenant quotas, queue-age/deadline policy, dead-letter
retention, operational pause/resume, and metrics-backed autoscaling need to be
added before public high-volume operation. These require product-level
fairness and retention decisions and must not be hidden in a local SQL
ordering change.
