# Phase 175 — Geographically Distributed Backup Infrastructure

Status: **Local dump/checksum tools exist; protected backup infrastructure is absent**

## Recommended design

Store encrypted backups in a separately administered account and region, with
immutable retention where required, least-privilege writers/readers, key
recovery, integrity manifests, and monitoring. Include PostgreSQL base/WAL
archives, object data, configuration, and audit evidence with distinct
retention. Replication is not backup; checksums are not authenticity proofs.

## Remaining work and why

Select backup provider, regions, retention, encryption, immutable controls,
and restore ownership; automate verification and isolated restores. This
remains open because no provider or production data inventory is configured.

## Acceptance criteria

- Backups are isolated from production credentials and destructive roles.
- Encryption keys and recovery access are tested without weakening controls.
- Integrity, retention, deletion, and restore are automated and monitored.
- Cross-region copies comply with data residency and legal hold requirements.
- A successful restore is demonstrated; checksum verification alone is not
  accepted as recovery evidence.

## Research references

- [Phase 121 Backup Integrity](phase-121-backup-integrity-and-recovery-inventory.md)
- [PostgreSQL backup and restore](https://www.postgresql.org/docs/current/backup.html)
- [Amazon S3 replication](https://docs.aws.amazon.com/AmazonS3/latest/userguide/replication.html)
