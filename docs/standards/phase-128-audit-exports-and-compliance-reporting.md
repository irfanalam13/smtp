# Phase 128 — Audit Exports and Compliance Reporting

Status: **Audit schema exists; durable end-to-end audit and controlled exports pending**

## Repository baseline

PostgreSQL defines tenant-scoped `audit_events`, but application audit
integration is incomplete and the development control-plane adapter retains
some events in memory. There is no verified tamper-evident audit export,
evidence package, or framework-specific reporting service.

## Recommended design

Persist minimal security and administrative events with actor, tenant,
action, target, outcome, timestamp, request/correlation ID, and reviewed
metadata. Apply append-oriented permissions, integrity monitoring, retention,
restricted reads, and independent forwarding. Export jobs must be authorized,
tenant-scoped, bounded, encrypted in transit and at rest, and produce a
manifest/hash and provenance metadata. Reports should distinguish control
design from control operation and independently assessed evidence.

## Remaining work and why

Connect sensitive actions to durable audit, define a versioned schema and
integrity controls, and build reviewed export/report workflows. This remains
open because current schema does not establish operational completeness,
tamper resistance, or assurance.

## Acceptance criteria

- Sensitive identity, permission, billing, key, retention, and admin actions
  emit durable outcomes, including denied/failed actions where safe.
- Audit event insertion failures are visible and do not silently return
  success for actions requiring audit.
- Exports are least-privilege, scoped, rate-limited, integrity-verifiable, and
  independently access-logged.
- Report claims are traceable to evidence and do not assert certification.
- Retention/deletion rules for audit data are approved and exercised.

## Research references

- [Repository security audit logging](../security/audit-logging.md)
- [Phase 90 Audit and Compliance](phase-90-audit-compliance.md)
- [AICPA SOC Suite](https://www.aicpa-cima.com/resources/landing/system-and-organization-controls-soc-suite-of-services)
