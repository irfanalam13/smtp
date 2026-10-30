# Phase 81 — Webhook System

Status: **Durable mailbox outbox and webhook signature primitive exist; endpoint management and delivery service pending**

## Repository baseline

Migration `000010` adds a tenant/mailbox-scoped `mailbox_event_outbox` with
idempotency keys, lease fields, attempt counts, and retry scheduling. Rust
repository methods can claim, acknowledge, and retry outbox events, but no
webhook dispatcher or broker consumer is running. Phase 81 adds a small Go
signing primitive implementing the Standard Webhooks HMAC message format over
the exact serialized payload and attempt metadata. It is not wired to
delivery, endpoint storage, or secret management.

## Delivery contract

Webhook delivery is asynchronous and must never block SMTP acceptance,
delivery, or the authoritative database transaction. A stable event ID is
preserved across retries; each attempt gets a fresh timestamp and signature.
The signed bytes are exactly the bytes sent. Include a bounded, versioned
event envelope and minimal payload; do not send raw message bodies, credentials,
or unnecessary personal data by default.

Endpoint registration requires authenticated, tenant-scoped management,
event-type allowlists, bounded payload and concurrency, and auditable
activation/disablement. Before storing a destination, reject loopback,
link-local, private, multicast, reserved, and metadata-service IP ranges;
resolve and pin/check destination addresses on every connection to resist DNS
rebinding; disable redirects or revalidate every redirect target; allow only
approved schemes/ports; and enforce egress controls at the network layer.
Never forward internal credentials or tenant credentials to endpoints.

Use signed headers compatible with [Standard Webhooks](https://www.standardwebhooks.com/):
event ID, attempt timestamp, and exact body are HMAC-SHA256 signed. Secrets
must be random, shown only at creation/rotation, encrypted through a
production KMS/secret-provider boundary, and never stored or logged in
plaintext. Consumers should compare signatures in constant time and reject
stale timestamps, while still deduplicating by stable event ID.

Retries use bounded exponential backoff with jitter and documented
retryable/non-retryable outcomes. A delivery attempt is durable before
network I/O or is safely reconstructable from a lease; acknowledgement is
idempotent. Exhausted/poisoned deliveries enter a tenant-isolated dead-letter
state with operator replay, retention, and audit controls. Replay retains
event identity and is restricted by tenant and permission.

## Remaining work and why

Build endpoint/subscription CRUD, encrypted secret provisioning/rotation,
event filtering, an isolated dispatcher, SSRF-resistant HTTP transport,
attempt history, bounded retry/DLQ/replay, delivery metrics, rate limits, and
integration tests with a controlled local receiver. Define privacy and
retention for event payloads and endpoint responses.

This remains open because the existing mailbox outbox is only a durable
primitive and is not a webhook service. The signing helper cannot safely
deliver until KMS-backed secret handling, destination validation, persisted
subscriptions, tenant authorization, retry workers, and production egress
controls are present.

## Acceptance criteria

- Event creation is atomic with source-state changes and consumers tolerate
  at-least-once delivery.
- Signatures cover event ID, current attempt timestamp, and exact wire bytes;
  test vectors and rotation overlap are supported.
- SSRF tests cover IPv4/IPv6 special ranges, DNS rebinding, redirects, and
  metadata endpoints; network egress is independently restricted.
- Retry, lease expiry, duplicate acknowledgement, poison event, DLQ, and
  tenant-scoped replay are tested under concurrent workers.
- Delivery does not block mail acceptance; payloads exclude sensitive data
  by default and logs redact URLs/secrets/responses.

## Research references

- [Standard Webhooks specification](https://github.com/standard-webhooks/standard-webhooks/blob/main/spec/standard-webhooks.md)
- [CloudEvents 1.0.2](https://github.com/cloudevents/spec/blob/v1.0.2/cloudevents/spec.md)
- [Transactional outbox ADR](../architecture/adr/phase-01/0013-event-architecture.md)
- [Phase 70 outbox and platform status](phase-70-mailbox-platform-integration.md)
