# Phase 168 — Distributed Object Storage

Status: **Object storage is not implemented; mail payloads use the current PostgreSQL prototype**

## Recommended design

When object storage is introduced, define object identity, tenant isolation,
encryption/key ownership, versioning, replication scope, consistency, lifecycle,
legal holds, deletion, and database-to-object reconciliation. Replication is
asynchronous and must be monitored; it does not automatically copy existing
objects or prove destination readiness. Keep metadata authoritative and
content-addressed/deduplicated storage tenant-safe.

## Remaining work and why

Select object service, region strategy, data class/residency, key management,
replication SLA, and recovery. This remains open because no production object
store or attachment lifecycle is configured.

## Acceptance criteria

- Object metadata and payload integrity are verified across replicas.
- Replication lag, failures, versioning, deletes, and backfill are observable.
- Encryption keys, IAM, retention/holds, and cross-account ownership are tested.
- Restore/reconciliation covers database metadata and all object versions.
- Public access is blocked; download authorization is tenant-bound and audited.

## Research references

- [Amazon S3 replication](https://docs.aws.amazon.com/AmazonS3/latest/userguide/replication.html)
- [Phase 46 Attachment Storage](phase-46-attachment-storage.md)
- [Phase 121 Backup Integrity](phase-121-backup-integrity-and-recovery-inventory.md)
