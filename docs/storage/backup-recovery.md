# Backup and recovery runbook

This repository defines a local logical-backup procedure; it does not provision
production backup infrastructure or claim an RPO/RTO.

## Create and verify a local backup

```bash
export DATABASE_URL='postgres://...'
make backup-postgres
make verify-postgres-backup BACKUP=backups/postgres-<timestamp>.dump
pg_restore --list backups/postgres-<timestamp>.dump
```

`backup-postgres.sh` uses host `pg_dump` when available and otherwise runs
`postgres:17` in a temporary container. Output files use mode `0600` and must
remain outside source control. Do not put credentials in the backup filename
or command history. A `.sha256` sidecar is generated after the dump succeeds.
The verification command detects file changes relative to that sidecar; it is
not an authenticity proof if both files can be replaced, and it does not prove
the dump can be restored. Protect the manifest separately when using backups
for security-sensitive recovery.

## Isolated restoration test

Restore only into a new disposable database:

```bash
createdb smtp_platform_restore_test
pg_restore --exit-on-error --dbname "$RESTORE_DATABASE_URL" backups/file.dump
```

Then run schema/readiness checks and representative synthetic queries. Remove
the disposable database only after recording the result. Never restore over a
production database as a routine test.

Logical dump checksums do not provide point-in-time recovery. PITR requires a
base backup plus a complete WAL archive, configured and tested separately.

## Production prerequisites

An approved deployment must define backup frequency, retention, encryption,
access roles, storage destination, point-in-time recovery requirements, and
restoration ownership. Managed PostgreSQL backups or WAL archiving may be
selected later. Backups contain confidential mail and tenant data and require
the same or stronger access controls as the source database.

Redis and object storage are not current production data stores. If introduced,
their persistence, versioning, lifecycle, and recovery procedures must be
documented separately.
