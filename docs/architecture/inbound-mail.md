# Inbound mail reception

The inbound path is opt-in and separate from the outbound queue. The existing
SMTP session remains loopback-only by default. An operator supplies an
`InboundAcceptor` to the session configuration; the PostgreSQL implementation
is `smtp_ingress::inbound::PostgresInboundStore`.

After DATA is completely read and dot-unstuffed, the acceptor validates every
recipient against an active mailbox whose domain is `verified`. Domain matching
is case-insensitive; local parts are preserved and no provider-specific
aliases are applied. Unknown, pending, disabled, malformed, or cross-mailbox
recipients are rejected. The current schema intentionally requires all
recipients in one SMTP transaction to resolve to the same mailbox.

The acceptor commits mailbox routing metadata, the exact raw RFC 5322 bytes,
bounded size, a conservative subject summary, and accepted recipients in one
PostgreSQL transaction. The SMTP session sends `250` only after that commit.
Storage failures return `451`, while routing failures return `550` and an
oversized message returns `552`. A SHA-256 fingerprint of sender, normalized
recipients, and raw bytes provides retry deduplication for duplicate SMTP
transactions; this is at-most-once insertion for identical retries, not an
exactly-once delivery guarantee.

Raw content is not returned by the mailbox list API or rendered by the
dashboard. The current dashboard displays only persisted metadata. HTML
sanitization, raw-message streaming, deletion jobs, and a production object
storage implementation remain deferred.

The ingress configuration exposes low-cardinality atomic metrics for active
connections, accepted/rejected/temporary outcomes, and total message bytes.
Connection logs contain only sanitized lifecycle information; message bodies,
recipients, and sensitive headers are not logged.
