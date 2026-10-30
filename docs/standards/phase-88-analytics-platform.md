# Phase 88 — Analytics Platform

Status: **Analytics package reserved; durable tenant-scoped ingestion and reporting pending**

## Repository baseline

`smtp-analytics` is reserved for tenant-scoped usage, delivery, and operational
analytics. Current readiness/usage data is owned by the Go API; no general
event consumer, warehouse, engagement tracking, or customer analytics API is
implemented. Event/outbox consumers and production identity remain incomplete.

## Recommended analytics architecture

Build derived analytics asynchronously from authoritative, versioned events
and usage facts. Keep source records authoritative and analytics rebuildable.
Separate customer-facing business metrics from internal operational
telemetry; never expose global infrastructure metrics to tenants. Define
metric names, units, time windows, attribution, deduplication, late-event,
correction, and retention semantics before presenting totals.

Collect only data needed for an approved metric. Engagement tracking such as
opens/clicks has privacy, consent, and image-proxy limitations and must be
opt-in/product-reviewed. Enforce tenant scope in ingestion and query paths,
bound report windows/group-by cardinality, and disclose freshness. Analytics
must not block acceptance, delivery, or webhook dispatch.

## Remaining work and why

Define customer and operational metric catalogs, event sources, privacy and
consent, attribution, units, retention, aggregation schedules, backfills,
correction handling, API/role policy, and warehouse/retention technology.
Then implement idempotent consumers, tenant-scoped aggregates, freshness/lag
monitoring, and reconciliation against source-of-truth rows.

This remains open because the package is only a placeholder and current
metrics are process-local or API-owned. Without durable event contracts and
metric semantics, dashboards can report inconsistent counts or leak
cross-tenant behavior.

## Acceptance criteria

- Every metric has a documented definition, unit, source, time zone, freshness,
  retention, and correction rule.
- Duplicate, delayed, replayed, and corrected events converge to consistent
  aggregates.
- Tenant isolation and authorization are tested on both ingestion and query.
- High-cardinality queries are bounded and costly reports are asynchronous.
- Customer analytics never block the SMTP delivery path or reveal other
  tenants' information.

## Research references

- [Analytics package boundary](../../smtp-analytics/README.md)
- [Phase 82 Event Bus and Messaging](phase-82-event-bus-messaging.md)
- [Repository data ownership](../architecture/phase-01/data-and-deployment.md)
