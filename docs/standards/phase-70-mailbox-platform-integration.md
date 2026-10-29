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
records. The Go HTTP control plane remains in-memory and is not connected to
that repository. SMTP ingress/egress, IMAP, object storage, queue/DSN workers,
production identity, distributed telemetry, representative load, failover,
backup recovery, and customer-facing API integration remain outstanding.

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
