# Phase 47 — MIME Parsing

Status: **Bounded standards-oriented parsing foundation implemented; inbound pipeline integration and conformance suite pending**

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

The `mail-parser` library is configured with explicit nesting and part bounds.
`parse_message` extracts decoded subject/address fields, plain and HTML bodies,
message/thread identifiers, inline and ordinary attachments, decoded payloads,
and bounded warning counts. Input, part count, attachment count, and total
decoded bytes are limited. MIME parser diagnostics are surfaced as warning
counts rather than silently treated as clean.

This is not yet a complete production ingestion pipeline. It needs RFC-indexed
fixtures, fuzzing, CPU/time budgets, worker isolation, nested-message policy,
message disposition/quarantine behavior, attachment scanner integration,
encoded-word/address edge coverage, and an asynchronous durable projection
writer. The parser is not an SMTP acceptor or a claim of full RFC conformance.

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
