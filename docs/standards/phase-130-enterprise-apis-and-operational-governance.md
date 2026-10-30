# Phase 130 — Enterprise APIs and Operational Governance

Status: **API foundations exist; enterprise lifecycle APIs and governance are not complete**

## Goal and scope

Provide stable enterprise APIs and an operational governance system for
region management, organizations, policies, retention, audit export,
licensing, and lifecycle. APIs must preserve tenant authorization, audit,
idempotency, versioning, and data protection.

## Repository baseline

The Go API has a limited `/v1` control-plane surface and OpenAPI documentation.
Phases 78–80 are not complete, and there is no complete enterprise API,
administrative governance workflow, or production operating model.

## Recommended design

Use versioned resource APIs with documented error semantics, pagination,
idempotency for mutations, request limits, consistent authorization, and
durable audit. Define API deprecation and compatibility policy, service
ownership, change control, access reviews, incident/change records, operational
readiness reviews, and customer communication. Enterprise APIs must not
silently bypass UI safeguards or policy workflows.

## Remaining work and why

Complete underlying tenant, identity, authorization, audit, retention,
region, and licensing services before adding broad administrative endpoints.
This remains open because API surface without complete service contracts and
operational ownership would create unsupported security and reliability
promises.

## Acceptance criteria

- OpenAPI contract covers authorization, errors, pagination, idempotency,
  audit behavior, and compatibility lifecycle.
- Tenant-isolation tests cover every enterprise operation and object reference.
- Privileged operations enforce approvals and immutable audit requirements.
- API availability, rate budgets, support, deprecation, and incident ownership
  are documented and exercised.
- Release sign-off includes legal/security review and evidence for claims made.

## Research references

- [OpenAPI Specification](https://spec.openapis.org/oas/latest.html)
- [Phase 78 REST API Platform](phase-78-rest-api-platform.md)
- [Phase 105 API, Audit, and Supply-Chain Security](phase-105-api-audit-supply-chain-security.md)
