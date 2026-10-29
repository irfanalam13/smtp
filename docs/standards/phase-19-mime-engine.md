# Phase 19 — MIME Engine

Status: **Not implemented; design boundary documented**

The MIME engine must parse RFC 2045/2046 bodies with strict resource limits:
maximum nesting depth, part count, header bytes, decoded bytes, boundary
length, and transfer-decoding work. It should expose text/plain and text/html
views while retaining attachments and inline resources as opaque, bounded
parts with content IDs and sanitized metadata.

It must never recursively expand unbounded content, trust filenames or
content types for safety, or log raw parts. MIME parsing consumes the Phase 20
message parser’s header/body representation and must not modify the original
message bytes.
