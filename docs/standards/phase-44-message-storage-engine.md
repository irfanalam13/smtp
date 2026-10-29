# Phase 44 — Message Storage Engine

Status: **Transactional PostgreSQL prototype; scalable blob backend pending**

## Contract

Store the original RFC 5322 message bytes losslessly. MIME parsing produces
bounded projections; it must not rewrite the canonical message. Separate
immutable content from mailbox-specific state so one message can have distinct
flags, labels, dates, and UIDs in different folders.

The current storage path is PostgreSQL `BYTEA` and is limited by the configured
message-size bound. This is intentionally a small-deployment/reference path.
Production storage should use tenant-isolated encrypted object storage,
streamed uploads/downloads, integrity hashes, lifecycle policies, and
versioned metadata. Database rows remain authoritative for visibility and
authorization; an uncommitted object must never become readable. Cleanup must
be retryable and must not delete an object still referenced by a message.

Compression is applied to immutable content in the storage adapter, with
explicit codec/version metadata and bounded decompression. It must not be
performed repeatedly on a hot read path. Content-addressed deduplication is
per tenant by default. Hash validation and exact reference counting precede
garbage collection. Attachment objects have separate scan/quarantine state
and authorization.

Message append must atomically allocate UID, reserve quota, persist message
metadata, and publish an outbox/change event. Retries require an idempotency
key. Backups cover metadata and objects consistently and have tested restore
procedures.

## Code and gaps

Migration `000008` adds a 32-byte optional content hash and metadata columns.
`MailboxStore::append` uses a transactional quota update and folder UIDNEXT,
and expunge releases the same raw-byte quantity. Raw message bytes are still
stored inline in PostgreSQL; there is no durable MIME parser integration,
compression, object store, deduplication transaction, outbox, attachment
reference lifecycle, or idempotent append key. Consequently this is not yet a
high-scale storage engine.

## Acceptance criteria

- Exact message bytes round-trip after process/database restart.
- Quota is correct under concurrent appends, expunge, and transaction failure.
- Object write/DB commit failures and retries cannot expose partial data or
  delete referenced objects.
- Duplicate idempotency keys return the original append result.
- Integrity, streaming bounds, decompression bounds, encryption-key rotation,
  backup, restore, and orphan cleanup are tested.
- Load tests document p50/p95/p99 latency, throughput, storage amplification,
  and recovery point/time under an explicit workload.
