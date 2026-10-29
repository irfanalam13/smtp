# ADR-0012: Durable acceptance and at-least-once delivery

- Status: Proposed
- Date: 2026-10-28

## Context

SMTP success transfers responsibility to the platform. Queue publication and object storage cannot be assumed atomic, and remote SMTP acceptance can be ambiguous.

## Decision

Commit an authoritative message/queue record, idempotency record, content reference or bounded content, and transactional outbox before returning success. Workers use leases and idempotent state transitions. Ambiguous remote outcomes are recorded as ambiguous and may retry under a bounded policy.

## Consequences

Acknowledged-message loss is prevented within the database durability contract. Duplicate remote delivery remains possible; exactly-once delivery is not claimed. A reconciler and crash tests are mandatory.
