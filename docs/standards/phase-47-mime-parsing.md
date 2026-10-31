# Phase 47 — MIME Parsing

Status: **Bounded canonical parsing and durable queue/inbound/mailbox/API
integration implemented; attachment scanning and broader conformance remain**

## Research baseline

- [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322) defines Internet message format.
- [RFC 2045](https://www.rfc-editor.org/rfc/rfc2045) defines MIME headers, transfer encodings, and charset behavior.
- [RFC 2046](https://www.rfc-editor.org/rfc/rfc2046) defines multipart and media-type structure.
- [RFC 2047](https://www.rfc-editor.org/rfc/rfc2047) defines encoded words in header text.
- [RFC 2231](https://www.rfc-editor.org/rfc/rfc2231) defines extended parameters, character sets, and continuations.
- [RFC 2183](https://www.rfc-editor.org/rfc/rfc2183) defines Content-Disposition.

Parsing produces a projection; original message bytes stay canonical and
unaltered. Text decoding and transfer-decoding are separate stages. HTML is
untrusted input and must be sanitized by a renderer; the parser does not make
HTML safe.

## Implemented code and gaps

The shared version-1 canonical model keeps SMTP envelope metadata separate
from RFC 5322 headers and retains exact raw bytes. The `mail-parser` library
extracts decoded subject/address fields, plain and HTML bodies, message/thread
identifiers, inline and ordinary attachments, decoded payloads, and bounded
warning counts. Input/header bytes and counts, MIME depth, part count,
attachment count, decoded bytes, aggregate transfer-decoding expansion,
indexed metadata size, and parser deadline are limited. SMTP and mailbox
callers use a shared four-worker blocking-parser pool; diagnostics are
surfaced as warning counts rather than silently treated as clean.

Queue, inbound, and mailbox storage persist versioned bounded metadata beside
the unchanged raw message. API-generated submissions persist matching indexed
fields and fold generated headers safely. Tests cover folded/encoded headers,
nested multipart, base64 and quoted-printable content, RFC 2231 filenames,
inline parts, malformed transfer encoding, size/depth/part bounds, and
PostgreSQL persistence.

This is a bounded supported profile, not full RFC conformance. It does not
enable SMTPUTF8/IDNA or archive extraction, and still needs attachment scanner
integration, renderer-side HTML sanitization, larger interoperability
fixtures, and projection rebuild workflows.

**Why this remains open:** Parsing is currently a bounded component, not a
production ingestion pipeline. It must be connected asynchronously to durable
message storage and attachment scanning, with resource isolation and malformed
input coverage, before its projections can be trusted by mailbox features.

## Acceptance criteria

- Nested multipart/alternative, multipart/related, mixed, and encapsulated
  `message/rfc822` fixtures preserve MIME roles and CID references.
- Base64, quoted-printable, RFC 2231 parameters, RFC 2047 headers, and legacy
  charsets are tested with both valid and malformed cases.
- Oversized input, excessive parts/depth, and decoded expansion stop within
  known memory/CPU budgets.
- Parsed text and attachments are derived from but never overwrite canonical
  message bytes.
