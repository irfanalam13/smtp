# Phase 70 — Mailbox Platform Integration

Status: **Repository foundations and isolated PostgreSQL integration tests exist; production service integration is not complete**

## Design contract

Storage, MIME projection, search, filters, synchronization, attachment access,
authentication, and observability must share tenant/mailbox authorization and
transactional lifecycle boundaries. A prototype or passing component test is
not proof of production readiness. Capabilities should be enabled only when
their workers, operational controls, and recovery evidence are deployed.

## Implementation and remaining

Rust repository primitives cover mailbox storage, permission checks, MIME
projection, rules, labels, Trash, drafts, quota, and durable change/outbox
records. The Go API now connects to PostgreSQL for mailbox listing and bounded
message metadata reads. Those routes authenticate PostgreSQL API keys bound to
active tenant users and enforce mailbox ownership/permission checks in
tenant-scoped SQL; startup requires PostgreSQL, readiness pings it, and
shutdown closes the pool. Other control-plane state remains in-memory.

Remaining work includes OIDC/JWT identity, durable control-plane writes,
SMTP/IMAP service adapters, object storage, outbox and retention workers,
centralized telemetry, and representative load/failover/restore evidence.
PostgreSQL-backed mailbox reads are a narrow vertical slice, not complete
customer-facing platform integration.

**Why this remains open:** Components and repository primitives are not yet
connected across the entire message lifecycle. Only mailbox reads use the
durable Go repository; identity, writes, protocol ingress/egress, background
workers, and operational recovery still need end-to-end integration. Without
those paths and evidence, consistent authorization, delivery, durability, and
production readiness cannot be claimed.

## Launch gate

- Production identity and PostgreSQL repositories are wired end-to-end.
- Every user-facing route and protocol path enforces the same authorization.
- Outbox, retention, archive, and recovery workers are deployed and monitored.
- Security, concurrency/load, migration, restore, and interoperability evidence
  meets approved launch criteria.

## Sources

- [Phase implementation status](../phase-status.md)
- [Backup and recovery runbook](../storage/backup-recovery.md)
- [Release readiness evidence](../validation/release-readiness.md)
