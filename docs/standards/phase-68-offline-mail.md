# Phase 68 — Offline Mail

Status: **In-memory reference queue exists; server-change reconciliation is being improved; durable client cache is not implemented**

## Design contract

Offline caches are client-owned replicas and must be encrypted by the client
platform. Mutations need stable idempotency IDs, retryable acknowledgements,
and explicit conflict policy. The server must not trust offline content or
client timestamps as authorization or canonical state.

## Implementation and remaining

The core reference queue holds pending upsert/delete operations and a cursor.
Server move and flag changes are applied to clean cached records; dirty records
are preserved for conflict handling. Acknowledge-through semantics prevent
new operations from being cleared while an earlier batch is in flight.
Durable encrypted cache, persistent operation IDs, retry/dead-letter handling,
conflict UI, and crash recovery remain client responsibilities.

## Acceptance criteria

- Acknowledging a sent batch cannot discard operations queued after that batch.
- Dirty local state is not silently overwritten by remote metadata changes.
- Cursor and pending mutations survive restart in a real client implementation.

## Sources

- [RFC 9051 — IMAP4rev2](https://www.rfc-editor.org/rfc/rfc9051.html)
- [RFC 8621 — JMAP Core and Mail](https://www.rfc-editor.org/rfc/rfc8621.html)
