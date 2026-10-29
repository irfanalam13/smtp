# Phase 69 — Storage Optimization

Status: **Attachment reference prototype compresses and deduplicates in process; durable message-body optimization is pending**

## Design contract

Compression, deduplication, and archival change confidentiality, quota,
retention, and recovery boundaries. Deduplication must be tenant-isolated and
must not disclose whether another tenant has stored matching content.
Compression and encryption codecs need versioned metadata and integrity
verification. Deletion requires reference-safe garbage collection.

## Implementation and remaining

The attachment prototype compresses and encrypts bytes and deduplicates only
within a tenant, but remains in-memory. Message bodies remain inline PostgreSQL
`BYTEA`; database-level partitioning, object storage, compression migration,
reference counts, garbage collection, and workload evidence are absent.

## Acceptance criteria

- Canonical message bytes can be verified after every tier/codec transition.
- Cross-tenant object existence and timing are not exposed.
- Garbage collection is reference-safe, retryable, and backup-aware.

## Sources

- [RFC 9051 — IMAP4rev2](https://www.rfc-editor.org/rfc/rfc9051.html)
- Storage codec and deduplication details are platform implementation policy,
  not standardized by the mail RFCs.
