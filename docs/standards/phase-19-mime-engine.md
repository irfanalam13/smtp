# Phase 19 — MIME Engine

Status: **Bounded `mail-parser` MIME profile implemented; broader scanning and
rendering controls remain deferred**

The MIME engine parses RFC 2045/2046 bodies with bounded nesting depth, part
count, aggregate header bytes/count, decoded bytes, attachment count, indexed
metadata size, and parser deadline. It exposes text/plain and text/html views
and retains attachment/inline payloads as bounded parts with content IDs and
metadata. Attachment payloads remain in the exact raw message; metadata stores
their content type, filename, content ID, inline role, and decoded size.

It does not decompress archives or treat filenames/content types as safety
signals. It never logs raw parts. MIME parsing uses `mail-parser`'s bounded
RFC 5322/MIME representation and does not modify the original message bytes.
HTML remains untrusted and requires renderer-side sanitization; attachment
scanning and archive extraction are separate future capabilities.
