# Phase 94 — Super Admin Platform

Status: **No platform-wide administration console or break-glass control plane implemented**

## Repository baseline

The repository defines tenant-scoped control-plane principals and audit
foundations. It does not have a platform-operator identity/role, global tenant
search/admin console, infrastructure control API, support impersonation, or
announcement service. Application metrics are local foundations, not a
centralized infrastructure dashboard.

## Recommended privileged-access model

Keep platform operator access separate from tenant roles and customer
sessions. Require phishing-resistant MFA through the selected identity
provider, least-privilege just-in-time grants, ticket/incident reason, short
expiry, dual approval for high-impact actions, and append-only external audit.
Support tools should expose minimized diagnostics and never display raw
message bodies, API secrets, or authentication tokens by default.

Impersonation, if approved, must be visibly marked, time-bounded, scoped,
consented where appropriate, and fully audited; prefer delegated diagnostic
views that do not issue customer credentials. Destructive global actions need
blast-radius preview, typed confirmation, rate limits, and rollback/recovery.
Infrastructure admin belongs in provider IAM/observability consoles, not a
second application-level root surface.

## Remaining work and why

Define operator roles, identity, approvals, incident access, support
diagnostics, audit retention, customer notification, blast-radius controls,
and tenant privacy. Build only required support workflows with read-only
default access, then integrate infrastructure monitoring through existing
observability tooling.

This remains open because a global admin console concentrates cross-tenant
privilege and the repository lacks production IdP, durable audit, and
centralized telemetry. Implementing global access first would create a
high-impact target without the controls needed to constrain or investigate it.

## Acceptance criteria

- Operator access uses separate identity, MFA, least-privilege grants, expiry,
  approvals, and monitored break-glass.
- Every tenant-wide view/action has a reason, actor, scope, request ID, and
  tamper-resistant audit record.
- Sensitive message content and credentials are masked by default and access
  is separately authorized.
- Bulk actions have bounded blast radius, dry run, cancellation, and recovery.
- Security exercises validate operator compromise, session revocation, and
  incident audit completeness.

## Research references

- [Phase 74 Authentication and Identity](phase-74-authentication-identity.md)
- [Phase 90 Audit and Compliance](phase-90-audit-compliance.md)
- [NIST SP 800-53 Rev. 5](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final)
