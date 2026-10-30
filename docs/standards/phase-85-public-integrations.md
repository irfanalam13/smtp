# Phase 85 — Public Integrations

Status: **No customer-facing third-party integration framework implemented**

## Repository baseline

The platform has SMTP/DNS capabilities and reserved analytics/SDK areas, but
no Slack, Teams, Discord, Google Workspace, Microsoft 365, CRM, Zapier, Make,
or ERP integration adapters. Webhook delivery and general event consumers
remain incomplete.

## Recommended integration model

Prioritize a secure webhook/API developer contract before building a catalog
of vendor-specific connectors. Each connector must have a documented owner,
data-flow diagram, scopes, OAuth client model, token storage/rotation,
rate-limit behavior, retry/idempotency policy, deletion/revocation path, and
support boundary. Use official provider APIs and least-privilege delegated
scopes; do not collect provider passwords or request broad scopes without a
feature need.

OAuth authorization must follow current security best practice, including
strict redirect validation, state/CSRF defenses, PKCE where applicable,
token audience/resource restriction, secure secret storage, and revocation.
Connector callbacks are untrusted input. Validate signatures where offered,
deduplicate events, bound payloads, and keep integrations asynchronous so a
provider outage cannot block SMTP acceptance.

## Remaining work and why

Collect customer requirements and rank integrations by demand and maintenance
cost. Define supported data flows, consent and scopes, provider terms, privacy
and residency, incident responsibilities, OAuth lifecycle, sandbox tests,
and connector versioning. Implement one narrow integration at a time after
the webhook/event foundation is deployed.

This remains open because the requested provider list has no prioritization
or defined synchronization behavior. Each integration adds credentials,
privacy exposure, provider-specific outages, rate limits, and long-term
maintenance; implementing many connectors speculatively is not production
engineering.

## Acceptance criteria

- Every connector uses least privilege, encrypted token storage, rotation and
  revocation, with audit events.
- Provider webhooks/API results are authenticated, bounded, idempotent, and
  replay-safe.
- Tenant data cannot cross connector installations, and uninstall removes
  credentials and scheduled work.
- Provider throttling/outage/backfill behavior is tested and observable.
- Data use, consent, retention, residency, and provider terms are reviewed
  before launch.

## Research references

- [RFC 9700 — OAuth 2.0 Security Best Current Practice](https://www.rfc-editor.org/rfc/rfc9700.html)
- [Phase 74 Authentication and Identity](phase-74-authentication-identity.md)
- [Phase 81 Webhook System](phase-81-webhook-system.md)
