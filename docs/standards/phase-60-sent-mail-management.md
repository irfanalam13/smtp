# Phase 60 — Sent Mail Management

Status: **HTTP submissions reach the durable queue; sent-copy and SMTP/DSN integration remain pending**

## Research and contract

RFC 6409 separates message submission from relay and delivery; SMTP acceptance
is not proof of final delivery. RFC 5321 queue/relay outcomes and DSN data
provide delivery signals, while RFC 6154 identifies the `\Sent` mailbox role.
JMAP (RFC 8621) specifies a synchronized submission model.

Store the sent copy and submission/delivery state with idempotent linkage.
Track recipient outcomes independently because one submission can have mixed
results. Never report queued/accepted as delivered; preserve a relationship to
the existing queue/DSN identifiers and avoid duplicating transport logic.

## Implemented and remaining

Migration `000010` adds sent-submission and per-recipient event projections,
with constrained states and idempotency keys. Repository methods persist
submissions and idempotent recipient events with derived status. The
tenant-scoped HTTP submission endpoint now enqueues plain-text messages
atomically with recipients and an idempotency record. It does not yet write
Sent messages, consume queue events, or change outbound delivery.

Still required: SMTP submission integration and sent-copy transaction,
durable queue/DSN event consumer, recipient privacy/retention controls,
authenticated synchronization APIs, and end-to-end delivery failure tests.

**Why this remains open:** The HTTP submission endpoint only records durable
queue acceptance. It is not connected to SMTP AUTH/outbound delivery, sent
copies, or durable queue/DSN events. Until those adapters are wired, the
platform cannot consistently create a sent copy or distinguish accepted,
queued, and delivered outcomes.

## Acceptance criteria

- Submission acknowledgement and recipient delivery status are distinct.
- Retries do not create duplicate submissions or sent copies.
- Per-recipient terminal states derive from durable queue/DSN events.
- Every Sent listing is authorized and synchronized consistently.

## Sources

- [RFC 6409](https://www.rfc-editor.org/rfc/rfc6409.html)
- [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321.html)
- [RFC 6154](https://www.rfc-editor.org/rfc/rfc6154.html)
- [RFC 8621](https://www.rfc-editor.org/rfc/rfc8621.html)
