# ADR-0602: Defer Redis and object storage

- Status: Accepted
- Date: 2026-10-28

## Decision

Do not add Redis or an S3-compatible object store until an implemented service
has a concrete contract, lifecycle, tests, and operational owner. Redis is not
the authoritative store for durable mail or tenant state. Object storage will
be required for larger immutable content only when PostgreSQL payload limits
and workload justify it.

## Consequences

Local development and CI remain simpler and do not imply unsupported
durability, cache invalidation, bucket security, or cross-store atomicity.
Future adapters must add explicit consistency, retention, access, and
reconciliation rules.
