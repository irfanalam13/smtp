# Phase 20 — Message Parser

Status: **Bounded RFC 5322 projection implemented through `mail-parser`**

The shared parser provides bounded RFC 5322 header/body handling for inbound,
queued, mailbox, and API-generated messages. It preserves raw bytes separately,
handles folded headers and RFC 2047 presentation values, and retains malformed
transfer-encoding diagnostics. Header bytes/count, message size, nesting,
part count, decoding, metadata size, and parser time are bounded. Parsed
headers never authorize the SMTP envelope.

SMTPUTF8/IDNA and authorization-grade address normalization remain separate,
deferred end-to-end features. Transfer decoding and MIME projection are part of
the shared parser. Parser output is a versioned projection and never replaces
the immutable raw bytes retained for queueing, delivery, and audit.
