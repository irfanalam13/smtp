# Phase 123 — Point-in-Time Recovery

Status: **No WAL archiving, base-backup chain, or PITR exercise configured**

## Goal and scope

Recover the authoritative PostgreSQL cluster to a verified point before
accidental deletion, corruption, or application failure, while preserving
message and audit consistency. Logical `pg_dump` backups are useful for
portability but cannot be replayed with PostgreSQL WAL for PITR.

## Repository baseline

The repository documents local logical backup and isolated restore only.
There is no production base-backup schedule, WAL archive, recovery target
configuration, managed PITR setup, or measured recovery objective.

## Recommended design

Use the selected managed service or PostgreSQL base backups plus a continuous,
monitored WAL archive. Protect archive credentials and storage independently;
validate archive continuity and recovery target selection. Coordinate database
recovery with object payload versions, queue state, idempotency records, audit
events, identity, and API compatibility. Define a recovery point that avoids
replaying unsafe external side effects.

## Remaining work and why

Select a database service, WAL retention, encryption/key recovery, target
granularity, and restore operator. Run repeated restores to isolated clusters
and measure end-to-end RPO/RTO. This remains open because WAL archive and
managed database choices are not deployed.

## Acceptance criteria

- Base backups and uninterrupted WAL segments support documented recovery
  targets.
- Restore automation validates recovery target, schema/migrations, row counts,
  and application readiness in isolation.
- Recovery includes consistent treatment of queue leases and already-issued
  external SMTP side effects.
- Missing/corrupt WAL and expired recovery windows produce explicit alerts.
- RPO/RTO are measured and reported from recurring exercises.

## Research references

- [PostgreSQL continuous archiving and PITR](https://www.postgresql.org/docs/current/continuous-archiving.html)
- [Repository backup and recovery runbook](../storage/backup-recovery.md)
