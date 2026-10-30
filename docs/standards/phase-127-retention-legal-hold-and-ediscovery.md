# Phase 127 — Retention, Legal Hold, and eDiscovery

Status: **Mailbox-level hold metadata exists; policy enforcement and legal workflows pending**

## Repository baseline

Mailbox archive rows include `retention_until` and a `legal_hold` flag. This is
not a complete tenant policy engine, scheduled retention executor, immutable
hold, case-management system, or eDiscovery workflow. Audit/compliance
requirements also remain open.

## Recommended design

Define retention by data class, tenant contract, jurisdiction, and lawful
purpose. Legal hold must identify an authorized matter, scope, custodian,
reason, owner, start/end events, approvals, and review dates. Holds must
override ordinary deletion only for scoped data and be released through an
authorized, audited process. Discovery exports must be scoped, reproducible,
integrity-checked, access-controlled, and preserve chain-of-custody metadata.

Keep legal case details and privileged material in restricted stores. Avoid
using a generic boolean as the sole legal hold control. Ensure deletion,
backup expiry, replicas, caches, and exports have explicit handling.

## Remaining work and why

Obtain legal requirements, define policy lifecycle and roles, and implement
audited scheduling, holds, searches, exports, and deletion reconciliation.
This remains open because existing schema fields do not enforce legal policy
or provide litigation-grade evidence.

## Acceptance criteria

- Retention and deletion rules are versioned, tenant-aware, and legally
  reviewed for applicable jurisdictions.
- Holds have scoped authorization, approvals, periodic review, release, and
  immutable audit trail.
- eDiscovery results record query scope, actor, time, export hash, and custody
  transfer without exposing unrelated tenants.
- Backup, replica, archive, and derived data behavior is explicitly covered.
- Tests prove hold-protected data is not purged and released data is eligible
  under policy.

## Research references

- [GDPR — Regulation (EU) 2016/679](https://eur-lex.europa.eu/eli/reg/2016/679/oj)
- [Phase 64 Retention Policies](phase-64-retention-policies.md)
- [Phase 90 Audit and Compliance](phase-90-audit-compliance.md)
