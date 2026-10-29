# Migration and schema operations

## Apply migrations

Set `DATABASE_URL` to a disposable or intended local database and run:

```bash
make migrate-up
make migrate-validate
```

The migration directory is mounted read-only into the pinned
`migrate/migrate:v4.18.3` image. The migration tool records applied versions and
prevents concurrent incompatible application. A fresh database receives all
migrations in lexical version order.

## Create and review a migration

1. Add the next unique numbered `.up.sql` and `.down.sql` files.
2. Keep the change forward-compatible with currently deployed code.
3. Use constraints and indexes that match an actual query or invariant.
4. Review lock duration and whether the operation is transactional.
5. Apply it to a fresh database and a copy containing synthetic data.
6. Run the queue/control-plane integration checks.
7. Never edit a migration after it has been applied to a shared database.

Destructive changes require explicit review. For live services, use
expand-and-contract: add compatible structures, deploy dual-compatible code,
backfill, switch reads/writes, and remove old structures later.

## Rollback and recovery

Application rollback does not automatically roll back schema changes. Prefer a
corrective forward migration. `make migrate-down` is limited to one migration
and must only be used on a disposable database. If a migration fails, preserve
the error and database state, inspect the migration table, and resolve with a
reviewed correction rather than deleting migration history.
