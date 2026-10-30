# Phase 89 — Reporting Engine

Status: **No scheduled customer reporting or export engine implemented**

## Repository baseline

The dashboard shows limited readiness and usage information. There is no
durable tenant analytics foundation, scheduled report service, custom
dashboard API, or CSV/Excel/PDF export workflow. The phase depends on
well-defined metrics and tenant authorization from Phase 88.

## Recommended reporting contract

Define reports over versioned analytics metrics rather than ad hoc joins
against live mail tables. Each report has an owner, tenant scope, time zone,
filters, freshness marker, and bounded cost. Scheduled reports use durable
jobs, idempotent delivery, tenant quotas, explicit recipients, retention, and
auditing; generated files are encrypted, expire, and require authorized
download.

CSV/Excel output must neutralize spreadsheet formula injection and preserve
correct quoting/encoding. PDF rendering must escape untrusted content and use
a sandboxed renderer without network or filesystem access. Do not include
message bodies or sensitive recipient data unless explicitly required and
authorized. Executive/compliance reports must state their data source and
limitations; they are not audit attestations.

## Remaining work and why

Define report catalog, custom query bounds, formats, scheduled delivery,
recipient authorization, time zones, freshness, export retention, cost
controls, cancellation, and support/audit operations. Implement only after
Phase 88 metrics are stable, with load tests and representative file
validation.

This remains open because there is no durable analytics source or report
definition yet. Export jobs can create sensitive persistent copies and
expensive queries; a simple dashboard screen does not provide the lifecycle,
security, or operational controls required.

## Acceptance criteria

- Every report is tenant-scoped and permission-checked at generation and
  download time.
- Jobs are durable, cancellable, bounded, idempotent, and observable.
- Exports expire, are encrypted at rest, and do not disclose secrets or
  unnecessary personal/message data.
- CSV/Excel formula-injection and PDF rendering risks are tested.
- Reports state freshness, filters, time zone, units, and known limitations.

## Research references

- [Phase 88 Analytics Platform](phase-88-analytics-platform.md)
- [OWASP CSV Injection](https://owasp.org/www-community/attacks/CSV_Injection)
- [Phase 66 Backup and Recovery](phase-66-backup-and-recovery.md)
