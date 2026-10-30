# Phase 159 — Enterprise AI Governance

Status: **No organization AI policy or governance console implemented**

## Repository baseline

The AI package is a placeholder and prior phases define only offline feature
and suggestion contracts. There are no organization-level AI policies,
approval workflows, model inventory, audit reports, or governance APIs.

## Recommended design

Provide tenant-admin controls for feature enablement, data categories,
providers, residency, retention, model versions, human approvals, and
prohibited uses. Enforce policy in service boundaries, not only UI. Maintain
an auditable model/use-case inventory, risk assessments, evaluations,
exceptions, incidents, and approval history. Separate tenant governance from
platform-wide safety controls; customer policy cannot disable baseline
security.

## Remaining work and why

Complete tenant isolation, RBAC, durable audit, model management, legal
applicability, policy APIs, and admin roles. This remains open because the
enterprise foundations and operational evidence are absent.

## Acceptance criteria

- AI use case, model, data category, purpose, owner, risk, and evaluation are
  inventoried.
- Policy is checked server-side for every inference and persisted action.
- Administrators can disable features and request deletion subject to
  retention/legal requirements.
- High-impact use cases require documented human approval and periodic review.
- Governance reports distinguish self-attestation from independent assurance.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 126 Compliance Scope and Governance](phase-126-compliance-scope-and-governance.md)
- [Phase 156 AI Privacy and Security](phase-156-ai-privacy-and-security.md)
