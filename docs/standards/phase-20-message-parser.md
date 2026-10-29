# Phase 20 — Message Parser

Status: **Not implemented; design boundary documented**

The parser will provide bounded RFC 5322 header/body handling for inbound and
queued messages. It must preserve raw bytes, parse CRLF header sections,
validate folding, unfold only syntactically valid continuation lines, decode
RFC 2047 presentation values, and retain malformed input diagnostics without
turning untrusted headers into authorization data.

Unicode normalization, address parsing, transfer decoding, and MIME parsing
remain separate APIs. Parsing limits and malformed-message policy must be
explicit and testable; parser output must never replace the immutable raw
message retained for delivery and audit.
