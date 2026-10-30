# Phase 121 — Backup Integrity and Recovery Inventory

Status: **Local PostgreSQL dump checksum verification added; protected production backups and restore evidence pending**

## Repository baseline and code slice

The repository has a manual `pg_dump` custom-format procedure. The backup
script now writes a SHA-256 sidecar after successful dump creation;
`scripts/verify-postgres-backup.sh <backup.dump>` checks the file against its
sidecar. This detects accidental corruption or mismatch after creation. It
does not authenticate the manifest, protect against an attacker who can
replace both files, prove that the dump restores, include object data, or
provide point-in-time recovery.

## Recommended design

Maintain an inventory of authoritative data, dependencies, owners,
classification, backup method, encryption/key owner, retention, recovery
priority, and restore validation. Treat mailbox payloads and audit data as
confidential. Keep backups isolated from source credentials, production
write access, and ordinary tenant administration. Store signed or separately
protected manifests in the approved backup platform.

## Remaining work and why

Select a production backup target and key policy, automate verification and
restore drills, and include every data store and external dependency. The
current local logical dump is not a production backup service and cannot
establish RPO/RTO.

## Acceptance criteria

- Every authoritative data set has an owner, backup method, retention, and
  recovery procedure.
- Backup integrity and catalog checks are automated and alert on failure.
- Restore tests verify schema, representative records, and cross-store
  references in an isolated environment.
- Backup access is least-privilege and segregated from production write access.
- The distinction between checksum verification and restore validation is
  explicit in operator guidance.

## Research references

- [PostgreSQL backup and restore](https://www.postgresql.org/docs/current/backup.html)
- [Repository backup and recovery runbook](../storage/backup-recovery.md)
- [Phase 66 Backup and Recovery](phase-66-backup-and-recovery.md)
