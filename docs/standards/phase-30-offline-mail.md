# Phase 30 — Offline Mail Support

## Research baseline

Offline clients need a local immutable cursor, a bounded cache, a pending
mutation queue, and recovery when the server invalidates a cursor. Local data
is untrusted and must be encrypted by the client/device layer; the server must
not assume an offline cache is authoritative.

## Implemented foundation

`OfflineMailbox` applies server deletions, tracks a synchronization cursor,
queues local upserts/deletions, and clears operations only after explicit
acknowledgement.

## Production requirements

- Add durable client storage and encryption outside this server-side core.
- Define conflict resolution using message version/actor metadata.
- Support retry, deduplication, dead-letter operations, and full resync.
- Bound cache size and protect message content at rest.
- Add property, crash-recovery, and multi-device synchronization tests.

