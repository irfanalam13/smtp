# Database and storage architecture

Phase 06 builds on the existing PostgreSQL implementation. The current
platform has executable durable queue, control-plane, and opt-in inbound-mail
metadata, so PostgreSQL is the only approved local storage dependency.

| Store          | Current responsibility                                                                   | Authority     |
| -------------- | ---------------------------------------------------------------------------------------- | ------------- |
| PostgreSQL     | Queue state, message payloads, control-plane metadata, inbound messages, migration state | Authoritative |
| Redis          | Not implemented; no approved current consumer                                            | N/A           |
| Object storage | Not implemented; current payload limits fit the existing PostgreSQL prototype            | N/A           |
| Event broker   | Deferred by Phase 01; no durable adapter claimed                                         | N/A           |

PostgreSQL and object storage are not treated as one ACID transaction. If
object-backed message content is introduced later, metadata and object
reconciliation must be explicit. Redis must remain cache or ephemeral state
unless a later architecture decision assigns it a durable responsibility.

## PostgreSQL integration

The shared [database module](../../smtp-core/src/platform/database.rs) provides
validated `DATABASE_URL` configuration, bounded pool settings, an optional
statement timeout, safe connection-string `Debug` redaction, and an explicit
readiness query.

Services own their pools; the shared crate does not create a global pool.
`mail-queue` retains its domain-specific `Queue` pool and configuration because
its lease and retry behavior belongs to that service. Runtime credentials must
come from the environment or the Phase 04 secret provider and must never be
logged.

## Schema ownership

Existing migrations under [`smtp-storage/postgres/`](../../smtp-storage/postgres/)
are immutable and forward-only:

- `000001`: schema metadata;
- `000002`–`000003`: durable mail queue, idempotency, attempts, and terminal
  state compatibility;
- `000004`: tenant, domain, API-key, membership, and audit metadata;
- `000005`: nullable reverse-path support;
- `000006`: verified-domain inbound mailboxes and message persistence;
- `000007`–`000008`: mailbox folders/messages and mailbox permissions;
- `000009`–`000010`: search projections and mailbox workflows;
- `000011`: mailbox metadata sync/outbox journaling;
- `000012`: API-key creator membership binding for mailbox authorization;
- `000013`: explicit tenant- versus user-bound API-key classification.
- `000014`: inbound message/mailbox tenant-consistency constraint.
- `000015`: optional OIDC issuer binding for provisioned user identities.
- `000016`: API-key name uniqueness within an owner scope;
- `000017`–`000018`: tenant-scoped SMTP credentials and shared auth controls;
- `000019`: tenant/domain SMTP message-size policies;
- `000020`: versioned parsed-message metadata for queue, inbound, and mailbox
  records.
- `000021`: durable per-recipient outbound delivery states and leases.
- `000022`: shared PostgreSQL inbound rate-limit buckets keyed by HMAC digest.
- `000023`: SMTP peer IP, receiving listener, and TLS state for accepted
  inbound messages.
- `000024`: safe inbound authentication summaries and quarantine status.
- `000025`: authorized DMARC report aggregation, report outbox, and normalized
  report ingestion with 90-day retention.
- `000026`: mailbox append idempotency records, per-user/mailbox key scope,
  request digest validation, and 30-day expiry.

The database enforces primary keys, foreign keys, uniqueness, bounded lengths,
state checks, and payload-size checks. Timestamps use `TIMESTAMPTZ`; persistence
timestamps are assigned by PostgreSQL. Existing queue and inbound indexes
support due-work claims, lease recovery, tenant filtering, and mailbox reads.

Migration application uses `migrate/migrate:v4.18.3`, which serializes
concurrent migration attempts. Use `make migrate-up` for local application and
`make migrate-validate` against an isolated database. Do not rewrite an applied
migration. Prefer corrective forward migrations; down migrations are only for
disposable local databases.

## Transactions and consistency

Queue enqueue writes the message, recipients, queue row, and idempotency record
in one PostgreSQL transaction. Mailbox append writes its message, raw-byte
quota charge, UID allocation, and scoped idempotency result atomically. Claim
and lease updates use row locks with
`FOR UPDATE SKIP LOCKED`; completion verifies the lease token. External event
publication is not part of that transaction and is not claimed durable by this
phase.

Inbound persistence stores routing metadata and raw content in one database
transaction before SMTP acceptance. The current prototype bounds raw content at
10 MiB. It does not claim object-storage streaming or cross-store atomicity.
Migration `000020` stores bounded RFC 5322/MIME metadata as a rebuildable
projection; the exact raw bytes remain the canonical message payload.

## Security and lifecycle

Local PostgreSQL is bound to `127.0.0.1`. Production database TLS, role
separation, encryption, and managed credentials remain deployment/security
responsibilities and must not be replaced with a static superuser connection.
Message bodies and raw inbound content are confidential; they must not appear
in SQL logs, diagnostics, fixtures, or backups copied into development.

Current cleanup semantics are explicit row deletion or status transitions owned
by the relevant service. Backup retention, legal retention, and customer
deletion policy are unresolved product/operations decisions; this phase does
not invent them.

## Remaining implementation

Migration `000012` binds newly issued API keys to their creating tenant member;
older unbound keys are not authorized for mailbox reads and must be rotated.
The Go API authenticates and proxies mailbox HTTP requests to the Rust
`mailbox-service`; the Rust service owns mailbox persistence and business
invariants. Other control-plane repositories, durable domain read models,
production retention, encrypted backup operations, and conditional
object-storage/Redis adapters remain deferred. Mailbox append-key expiry is
pruned in bounded batches on append and by the mailbox service's scheduled
cleanup loop for quiet mailboxes. The inbound projection worker consumes
accepted/quarantined events transactionally; it does not provide generic event
delivery or production monitoring.
