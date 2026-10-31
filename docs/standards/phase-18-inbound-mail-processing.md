# Phase 18 — Inbound Mail Processing

Status: **Opt-in bounded durable acceptance with same-tenant atomic mailbox
fan-out implemented; production inbound profile remains incomplete**

The inbound acceptor validates bounded recipients, resolves only active
mailboxes on active tenants and verified domains, stores exact raw message
bytes, envelope metadata, and the versioned RFC 5322/MIME projection
transactionally, and returns SMTP success only after durable commit. One
transaction can fan out atomically across multiple mailboxes in the same
tenant. Cross-tenant batches fail closed. Runtime wiring is disabled unless
`SMTP_INBOUND_ENABLED=true` is set with PostgreSQL configured.

The acceptor resolves primary and active alias addresses, reserves mailbox
quota transactionally, optionally scans through clamd's bounded INSTREAM and
SpamAssassin's bounded CHECK protocol, persists peer/listener/TLS metadata, and
inserts one retry-safe mailbox outbox event in the same transaction as each
durable copy. Public listener configuration requires both scanners and a
shared HMAC key for PostgreSQL-backed IP/domain rate limits.

Inbound now uses the M07 SPF/DKIM/DMARC evaluator and aggregate report event
capture; report generation/ingestion is implemented under M07. Remaining
production work includes managed DKIM domain controls, operational recovery
and monitoring for the inbound mailbox projection worker, and object storage.
The current duplicate
fingerprint treats byte-identical envelope submissions as retries because
SMTP supplies no supported idempotency token; identical intentional
resubmission can therefore be deduplicated. Raw message storage remains
separate from parsed display fields and is never rewritten by parsing.
