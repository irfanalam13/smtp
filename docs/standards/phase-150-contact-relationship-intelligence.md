# Phase 150 — Contact Relationship Intelligence

Status: **No communication graph, VIP inference, or relationship model implemented**

## Repository baseline

Mailbox data contains sender/recipient information, but there is no contacts
product, relationship graph, user consent flow, or behavior model. Communication
frequency is sensitive personal data and is not equivalent to trust.

## Recommended design

Make relationship insights user-controlled and mailbox-scoped. Derive only
minimal, explainable aggregates (for example, recent interaction count) with
short retention and clear reset/delete controls. Do not infer trustworthiness,
employment status, or user relationships for other tenants. Spam/phishing
trust must continue to rely on security signals, never frequency or a "VIP"
score alone.

## Remaining work and why

Define data-use purpose, contact ownership, sharing behavior, retention,
consent, and safe relationship categories. Validate privacy and fairness
impact before implementing an interaction graph. This remains open because
no contacts service, governance, model, or user controls exist.

## Acceptance criteria

- Insights are visible only to authorized mailbox users and cannot cross
  tenant/mailbox boundaries.
- Users can inspect, correct, reset, and disable derived relationships.
- Interaction frequency is not treated as sender authentication or malware/
  phishing clearance.
- Retention and deletion cover source events and derived aggregates.
- No individual-level relationship data is exported to analytics without
  explicit purpose and access approval.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 90 Audit and Compliance](phase-90-audit-compliance.md)
- [Phase 134 Phishing Detection](phase-134-phishing-detection.md)
