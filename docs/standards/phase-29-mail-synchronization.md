# Phase 29 — Mail Synchronization

## Research baseline

IMAP synchronization is driven by mailbox state, UIDs, UIDVALIDITY, and
incremental changes (RFC 9051 and RFC 7162). A synchronization token is a
projection cursor, not an authorization credential. Changes must be replayable
and bounded.

## Implemented foundation

`SyncLog` assigns monotonic tokens to upsert, delete, move, and flag changes
and supports bounded `since(token, limit)` reads. This is a process-local
reference log and does not replace a durable change journal.

## Production requirements

- Persist changes with mailbox and tenant scope.
- Retain history long enough for client recovery and expose reset behavior when
  a cursor is too old.
- Make mutations idempotent and define conflict ordering explicitly.
- Add backpressure, pagination, compaction, metrics, and audit logging.

