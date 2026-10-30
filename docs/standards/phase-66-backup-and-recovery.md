# Phase 66 — Backup and Recovery

Status: **Local PostgreSQL logical backup/isolated restore procedure documented; production backup service and PITR evidence pending**

## Design contract

Logical dumps support portability but do not by themselves provide a low
recovery-point objective. Point-in-time recovery requires a tested base-backup
and WAL-archive chain, protected credentials, encryption, immutable retention,
and isolated restore validation. Mail backups are sensitive data.

## Implementation and remaining

The [backup/recovery runbook](../storage/backup-recovery.md) documents local
dump inspection and restoration into a disposable database. No managed backup
target, WAL archiving, encryption/key lifecycle, restore orchestration,
application-consistent object-store snapshots, or approved RPO/RTO exists.

**Why this remains open:** The runbook documents a local manual procedure; it
does not establish protected production backups or point-in-time recovery.
Production readiness requires recurring isolated restore exercises across
database and object data, with measured recovery objectives and secured keys.

## Acceptance criteria

- Recovery tests verify both database and object-store referential integrity.
- RPO/RTO are measured from recurring isolated restore exercises.
- Backup access, retention, encryption, and deletion are independently
  audited.

## Sources

- [PostgreSQL Backup and Restore](https://www.postgresql.org/docs/current/backup.html)
- [PostgreSQL Continuous Archiving and Point-in-Time Recovery](https://www.postgresql.org/docs/current/continuous-archiving.html)
