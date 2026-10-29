# ADR 0003: Database migrations

- Status: Accepted
- Date: 2026-10-28

## Context

The platform needs a reliable initial system of record with repeatable local and CI setup.

## Decision

Use PostgreSQL and versioned SQL migrations executed by `migrate/migrate:v4.18.3`.

## Alternatives considered

An ORM-only schema or an embedded database would reduce initial setup but weaken operational parity and reviewability.

## Consequences

Schema changes are reviewable and repeatable. A fresh PostgreSQL instance is required for integration checks.
