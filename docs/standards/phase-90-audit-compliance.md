# Phase 90 — Audit and Compliance

Status: **Audit table and security guidance exist; durable append-only audit operations and compliance evidence pending**

## Repository baseline

Migration `000004` defines tenant-scoped `audit_events` with actor, action,
target, metadata, and timestamp. The Go development store still keeps audit
events in memory; no complete durable writer or independently protected
append-only sink is deployed. Mailbox phases add retention metadata/legal
holds and guarded purge primitives, but scheduled enforcement and
jurisdictional policy remain pending. The repository has baseline security
and audit guidance, but no GDPR tooling, HIPAA program, or SOC 2 report.

## Recommended compliance boundary

Implement durable, append-oriented audit records for security-sensitive
identity, membership, permission, billing, key, retention, export, and
administrative actions. Record actor, tenant, action, target, request/correlation
IDs, outcome, and minimal reviewed metadata. Do not log secrets, message
bodies, or unnecessary personal data. Restrict write/read access, protect
integrity, define retention and access review, and forward audit events to an
independently controlled sink. An audit row is evidence of an application
event, not proof that the action was legally compliant.

Privacy operations require a data inventory, controller/processor roles,
lawful basis, data-subject request workflow, export/deletion limits, retention,
legal holds, breach response, and jurisdictional counsel. Deletion must
respect applicable holds and preserve a documented tombstone/audit record
without retaining unnecessary personal data. HIPAA readiness requires
applicability analysis, risk analysis, safeguards, workforce/process controls,
and eligible business associate agreements; the codebase alone cannot make
the service compliant. SOC 2 is an independent assurance engagement; do not
claim certification based on checklists or source code.

## Remaining work and why

Connect all sensitive control-plane operations to durable audit; harden
append-only storage and access; define audit schema/version, retention,
integrity verification, export, and monitoring. Create privacy/data lifecycle
controls with legal review. Determine whether GDPR, HIPAA, or SOC assurance
applies based on actual users, data, contracts, geography, and operating
model; prepare evidence and independent audits where required.

This remains open because a schema and policy document do not provide
complete, tamper-resistant operational audit or legal compliance. Regulatory
scope depends on the service and customer context, and certifications require
organizational controls and independent assessment beyond implementation work.

## Acceptance criteria

- Security-sensitive actions produce durable, tenant-scoped audit events with
  access and integrity controls; failures are explicit and operationally
  visible.
- Audit payloads are minimized and contain no credentials, tokens, message
  bodies, or unreviewed sensitive data.
- Retention, legal hold, export, correction, deletion, and subject-request
  workflows are documented, authorized, testable, and jurisdiction-reviewed.
- Audit access, retention, integrity checks, incident response, and restore
  are regularly exercised.
- No GDPR/HIPAA/SOC 2 compliance or certification claim is made without
  applicable legal assessment and required independent evidence.

## Research references

- [GDPR — Regulation (EU) 2016/679](https://eur-lex.europa.eu/eli/reg/2016/679/oj)
- [HHS HIPAA Security Rule resources](https://www.hhs.gov/hipaa/for-professionals/security/index.html)
- [AICPA SOC Suite of Services](https://www.aicpa-cima.com/resources/landing/system-and-organization-controls-soc-suite-of-services)
- [Repository audit logging policy](../security/audit-logging.md)
- [Phase 64 Retention Policies](phase-64-retention-policies.md)
