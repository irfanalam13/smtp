# Phase 46 — Attachment Storage

Status: **Encrypted chunked reference store implemented; durable object storage and download service pending**

## Research baseline

- [RFC 2045](https://www.rfc-editor.org/rfc/rfc2045), [RFC 2046](https://www.rfc-editor.org/rfc/rfc2046), and [RFC 2231](https://www.rfc-editor.org/rfc/rfc2231) define MIME body encodings and extended parameters.
- [RFC 2183](https://www.rfc-editor.org/rfc/rfc2183) defines content disposition; filenames are metadata, not trusted paths.
- XChaCha20-Poly1305 provides authenticated encryption in the implemented reference store. Production keys must be generated and managed by a KMS/HSM or secret manager, not stored beside ciphertext.

## Storage contract

Uploads have sequential chunks, per-chunk and total-size bounds, validated
filename/media-type metadata, and a mandatory scanner at completion. Infected
or scanner-failed content is not committed. Content hashes are tenant-scoped;
equal bytes can share storage only within the tenant. Encryption binds the
tenant ID as associated data. Each encrypted object records a key version so
key rotation can preserve reads of older content until re-encryption finishes.

Download requires tenant authorization, uses bounded range/chunk reads, and
returns a standards-safe `filename*` content-disposition value. HTML, SVG,
scripts, and other active formats are never rendered by the preview path.
Text preview currently supports only `text/plain`. Malware verdicts,
quarantine, deletion, retention, audit, and signed URLs belong to the durable
service adapter.

## Implemented code and gaps

`AttachmentStore` now models sequential chunk upload, tenant-scoped key
selection, authenticated encryption, gzip compression, same-tenant
content-addressed deduplication, integrity verification, bounded downloads,
scanner injection, and plain-text previews. It is an in-memory reference
implementation: process restart loses all records, the key map is local
memory, and upload chunks are buffered in memory. It does not claim durable
secure storage or production key management.

Production completion requires an encrypted object-store implementation,
streaming without whole-object buffering, persistent upload sessions, quota
reservation, scanner timeout/isolation and quarantine states, tenant-aware
metadata transactions, signed expiring downloads, HTTP range controls, audit
events, retention/legal hold, key rotation/re-encryption, and failure/load
testing.

**Why this remains open:** The current implementation keeps attachment data
and key material in process memory, so restart loses state and whole-object
buffering limits scale. A durable download service also needs persistent
authorization, scanning, retention, and key lifecycle rather than only a
working reference-store algorithm.

## Acceptance criteria

- A chunk retry cannot reorder or duplicate content; abandoned uploads expire.
- Cross-tenant IDs do not reveal attachment existence.
- Ciphertext tampering and wrong tenant/key fail closed.
- Compression expansion is bounded and content hash is verified after decode.
- Scanning is required before visible metadata is committed.
- Download headers prevent path traversal and active-content execution.
- Backups, deletion, key rotation, scanner outages, and object-store failures
  have tested recovery behavior.
