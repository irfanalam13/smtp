# ADR-0014: Tenant-scoped ownership and defense in depth

- Status: Proposed
- Date: 2026-10-28

## Context

Tenant data includes credentials, message metadata, raw content, events, analytics, and backups. A single missing filter can disclose or send on behalf of another tenant.

## Decision

Tenant context comes from authenticated identity. Owners write authoritative data; database constraints, service roles, application authorization, tenant-tagged events, encrypted content, and negative tests provide layered isolation. Request bodies cannot select the tenant.

## Consequences

Every query and event path carries tenant context, increasing test and migration discipline. PostgreSQL row-level security may be added when session context and repositories can enforce it; it does not replace application checks.
