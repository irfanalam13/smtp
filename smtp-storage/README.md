# SMTP storage assets

This directory owns the PostgreSQL schema migration history. Migration files
are grouped by the feature that introduced them under `postgres/`; their
numeric prefixes remain the single global migration order. Applied migrations
are immutable. Add new schema changes as the next numbered up/down pair in the
matching feature folder.

The `migrate` CLI expects a flat directory. `scripts/stage-migrations.sh`
collects the feature folders into a temporary flat directory for `make
migrate-up`, `make migrate-down`, and `make migrate-validate`.

## Feature folders

- `postgres/platform/`: platform metadata.
- `postgres/queue/`: durable queue schema and compatibility changes.
- `postgres/control_plane/`: tenants, domains, API keys, and identity metadata.
- `postgres/inbound/`: inbound mail routing, rate limits, and authentication
  results, including DMARC report ingestion.
- `postgres/mailbox/`: mailbox storage, search, workflows, sync, and
  idempotency.
- `postgres/smtp_auth/`: SMTP credentials and submission policy.
- `postgres/message/`: shared canonical message metadata.
- `postgres/delivery/`: outbound recipient delivery state.
