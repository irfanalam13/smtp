# Phase 26 — Search and Indexing

## Research baseline

Search must be derived from normalized message headers and decoded body parts,
not from raw-byte substring matching. IMAP SEARCH (RFC 9051) defines mailbox
criteria and result semantics; full-text ranking and attachment metadata are
product extensions. Indexes must be rebuilt from durable messages and treated
as disposable projections.

## Implemented foundation

`shared-types` now provides a bounded `SearchDocument`, metadata/body and
attachment-name criteria, deterministic result scoring, upsert, and removal.
The implementation is an in-process reference index and is not yet a durable
PostgreSQL/GIN or external search backend.

## Production requirements

- Parse MIME and headers before indexing.
- Store tenant/mailbox authorization in every index row.
- Use PostgreSQL full-text/trigram indexes or a dedicated search cluster.
- Make indexing asynchronous, idempotent, replayable, and observable.
- Never index quarantined or unauthorized content.
- Add ranking, pagination, retention, and reindex jobs.

