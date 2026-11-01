# Inbound mail reception

The inbound path is opt-in and separate from the outbound queue. It is wired
into the SMTP executable only when `SMTP_INBOUND_ENABLED=true`; the default
listener remains loopback-only. The PostgreSQL implementation is
`smtp_ingress::inbound::PostgresInboundStore`.

After DATA is completely read and dot-unstuffed, the acceptor validates every
recipient against an active primary address or active explicit alias on an
active tenant and `verified` domain. Domain and local-part matching follow the
canonical case-insensitive mailbox-address index; plus-addressing and other
provider-specific rewrites are not applied. Unknown, pending, disabled,
malformed, or cross-tenant recipient batches are rejected with the same
generic recipient failure. Batches may fan out to multiple mailboxes when
every mailbox belongs to the same tenant. The product policy does not permit
cross-tenant fan-out.

The acceptor commits one mailbox-scoped copy for each distinct mailbox, with
routing metadata, the RFC 5321-required trusted `Received` trace followed by
the accepted RFC 5322 bytes, bounded size, a conservative
subject summary, and that mailbox's envelope recipients, in one PostgreSQL
transaction. Fan-out is all-or-nothing. The SMTP session sends `250` only
after that commit; any recipient, database, or uniqueness failure leaves no
partial copies. The `mailbox_id` and `tenant_id` composite foreign key remains
the final database isolation guard.

A versioned, length-framed SHA-256 fingerprint of sender, normalized
recipients, and the accepted wire bytes provides duplicate retry detection.
The fingerprint is computed before the current-hop trace is generated, while
the stored message contains that trace. The stored sender, traced bytes, and
mailbox recipient list are compared before a duplicate is accepted, so a
digest match alone never merges different content. SMTP has
no caller idempotency token in the supported profile; therefore byte-identical
intentional resubmissions are indistinguishable from retries and may be
deduplicated. This is at-most-once insertion for the fingerprinted envelope,
not an exactly-once delivery guarantee.

Raw content is not returned by the mailbox list API or rendered by the
dashboard. The current dashboard displays only persisted metadata.
The mailbox byte quota is reserved transactionally before each copy is
inserted. Each accepted mailbox copy also creates an idempotent
`inbound.message.accepted` event in the existing mailbox event outbox in the
same transaction. The outbox has lease, acknowledge, and retry primitives for
consumers. The Rust mailbox service consumes accepted and quarantined events,
projects their raw content and parsed metadata into INBOX or Junk, allocates a
folder UID, and acknowledges the event in the same transaction. A failed
projection rolls back both the message and acknowledgment and schedules a
bounded exponential retry. After twelve failed attempts, the event is marked
dead-lettered and excluded from automatic processing. Operators must inspect
`last_error` and may explicitly replay one selected event with
[`scripts/replay-inbound-event.sql`](../../scripts/replay-inbound-event.sql).
Logical quota is reserved by SMTP acceptance and is not charged again during
projection.

Each durable copy stores the SMTP peer IP, receiving listener name, and TLS
state with the envelope and traced message. The receiving hostname, peer IP,
transport, and timestamp in the prepended trace are generated only from
validated server-side values. Existing `Received` fields remain below the new
field and are never trusted as the current hop. The original accepted bytes
are used for SPF/DKIM/DMARC and scanner evaluation so adding the trace does not
break existing signatures.

When configured, ClamAV scanning uses the bounded clamd `INSTREAM` protocol
and SpamAssassin uses its bounded `CHECK SPAMC/1.5` protocol. Malware and spam
rejections receive permanent SMTP failures; unavailable scanners receive a
temporary failure. Non-loopback inbound startup requires both scanner endpoints
and a shared HMAC key for PostgreSQL-backed per-IP/per-domain hourly rate
limits. Rate-limit keys are pseudonymized and stale rows are pruned in bounded
batches. The per-IP bucket counts attempted DATA transactions, including
messages later rejected by policy; per-domain buckets are reserved after
scanning as part of the durable acceptance transaction.

Inbound acceptance runs SPF, DKIM, and DMARC through the `mail-auth` standards
implementation. The system resolver uses bounded timeouts, positive and
negative TTL caches, and CNAME handling; the adapter caps DKIM signatures at
four per message, while the SPF evaluator enforces RFC 7208 lookup limits and
the RFC 9989 DMARC tree walk is capped by the library. DNS temporary errors in
DMARC evaluation return SMTP `451`. A failing `p=reject` message receives
`554`; `p=quarantine` is durably stored with status `quarantined` and a
quarantine outbox event. Safe SPF/DKIM/DMARC result tokens are persisted with
the original raw message. RSA-SHA1 DKIM signatures are excluded from DMARC
alignment under RFC 8301.

HTML sanitization, raw-message streaming, deletion jobs, and production object
storage remain deferred. DMARC aggregate reports are stored without raw message
content or envelope recipients, generated daily through the report worker, and
ingested as normalized records by the bounded XML/gzip import command. The
inbound projection consumer is wired in `mailbox-service/src/inbound_projection.rs`;
lease recovery, replay/reconciliation tooling, tenant key management and
RFC-indexed interoperability evidence remain M07 work.

The ingress configuration exposes low-cardinality atomic counters for active
connections, accepted/rejected/temporary outcomes, quota, malware and spam
rejections, IP/domain rate limits, and total message bytes. These counters are
process-local. Connection logs contain only sanitized lifecycle information;
message bodies, recipients, and sensitive headers are not logged.

Greylisting is not enabled. The older simplified authentication functions in
`smtp-core/src/authentication/spf_dkim.rs` remain reference helpers and are not used for
inbound enforcement.
