# Message lifecycle and delivery semantics

## States

Internal states are precise; customer-facing status is a stable projection and must not expose implementation-only states.

```text
received → validating → accepted → persisted → queued → scheduled
scheduled → attempting
attempting → delivered_to_next_hop
attempting → temporarily_deferred → scheduled
attempting → permanently_failed → bounce_pending
bounce_pending → bounce_processed
queued/scheduled/temporarily_deferred → administratively_suppressed
queued/scheduled/temporarily_deferred → expired
```

`delivered_to_next_hop` is terminal for the platform's outbound transport only. It does not assert final mailbox placement. `administratively_terminated` is an operator-controlled terminal state and must carry an audited reason.

## Durable acceptance protocol

1. Authenticate and authorize the tenant, sending identity, recipients, quota, and policy snapshot.
2. Parse and bound the message; reject malformed or over-limit input before allocating unbounded resources.
3. Write the authoritative acceptance record, envelope metadata, idempotency key, content reference or bounded content, and an event/outbox row in one PostgreSQL transaction.
4. Commit. Only after commit return SMTP `250` or a successful API response.
5. A scheduler publishes/claims work from the authoritative queue state. If an event or object-storage upload is missing, a reconciler repairs or quarantines it; it does not invent success.

The repository's current PostgreSQL queue is the accepted initial mechanism for bounded messages. Production-sized raw content should move to encrypted object storage using a two-phase state (`content_pending`/`content_ready`) and an outbox/reconciliation process. Object storage and queue publication are never treated as one atomic commit.

If PostgreSQL is unavailable or the commit fails, return a temporary SMTP failure/API error and do not acknowledge. If a client retries after an ambiguous response, a tenant-provided idempotency key plus content fingerprint prevents avoidable duplicate acceptance; it cannot eliminate duplicate remote delivery.

## At-least-once and ambiguous outcomes

Workers claim with a lease, perform remote delivery outside the database transaction, and commit a result only with the lease token. A crash before completion causes lease expiry and retry. If the remote server accepted the message but the connection failed before a final response was recorded, the outcome is **ambiguous**: retry is permitted under policy but may duplicate delivery. The platform records this explicitly and never labels it a confirmed success.

Exactly-once delivery across arbitrary SMTP servers is not achievable because the remote acceptance and local state update are not one transaction. Duplicate suppression is therefore best effort: stable message IDs, idempotent local transitions, conservative retry budgets, and destination-aware policy.

## Retry policy

| Condition                                                    | Classification                               | Action                                                             |
| ------------------------------------------------------------ | -------------------------------------------- | ------------------------------------------------------------------ |
| DNS temporary failure, timeout, connection refusal, SMTP 4xx | Temporary                                    | Exponential backoff with full jitter and destination/tenant budget |
| SMTP 5xx, invalid recipient, policy rejection                | Permanent                                    | Terminal failure; classify bounce and update suppression policy    |
| TLS required but handshake/certificate fails                 | Policy/permanent until configuration changes | Do not silently downgrade; operator/customer correction            |
| Worker crash or lease expiry                                 | Ambiguous/internal                           | Reclaim after lease timeout; record attempt                        |
| Queue/content corruption or poison parser input              | Terminal/quarantine                          | Isolate, alert, require operator replay                            |

Initial provisional limits: retry for 24 hours, with per-destination concurrency, per-tenant fairness, queue age expiration, and a dead-letter/quarantine path. Limits must be tuned from delivery data. Priority is emergency/operator replay > normal transactional mail; priority cannot bypass abuse or destination limits.

## Event envelope

```json
{
  "event_id": "uuid",
  "event_type": "message.delivery.updated",
  "schema_version": 1,
  "tenant_id": "tenant-id",
  "aggregate_id": "message-id",
  "occurred_at": "RFC3339 timestamp",
  "producer": "delivery-worker",
  "correlation_id": "request-or-message-correlation",
  "trace_context": "optional W3C context",
  "payload": { "status": "delivered_to_next_hop", "attempt": 2 }
}
```

Events are append-only, at-least-once, replayable for a bounded retention period, and ordered only per aggregate where the store can provide it. Consumers deduplicate by `event_id` and tolerate schema-compatible additions. Message bodies, credentials, and full headers are not ordinary event payloads.
