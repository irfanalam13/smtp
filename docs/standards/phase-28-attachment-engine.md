# Phase 28 — Attachment Engine

## Research baseline

MIME media types and transfer encodings follow RFC 2045 and content
disposition follows RFC 2183. Filenames are untrusted metadata. Attachment
content must be streamed, size-limited, scanned, and stored separately from
message metadata.

## Implemented foundation

`AttachmentStore` validates filenames/content types, scans through an injected
scanner, computes SHA-256 content identity, deduplicates bytes, and exposes
bounded chunk reads. The scanner contract deliberately has no permissive
default: deployment must provide a real scanner.

## Production requirements

- Persist metadata and content references transactionally.
- Use object storage with tenant isolation and signed, expiring downloads.
- Enforce per-message, per-tenant, and aggregate quotas.
- Add antivirus/sandbox verdicts, quarantine, rescanning, and audit events.
- Generate previews asynchronously and never execute active content.

