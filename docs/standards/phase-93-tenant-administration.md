# Phase 93 — Tenant Administration

Status: **Partial domain/key/mailbox views exist; tenant administration workflows are incomplete**

## Repository baseline

The dashboard displays limited domains, API keys, mailboxes, usage, and
message metadata. Some state is still process-local; only selected mailbox
reads and API-key paths use PostgreSQL. There is no complete tenant admin
experience for users, billing, quotas, storage, policies, integrations, audit,
and analytics. The current dashboard also uses a local development identity.

## Recommended administration boundary

The dashboard is an untrusted client. All reads and writes must use
authenticated tenant-scoped APIs, with server-side permissions rechecked for
each resource and action. Never put durable authorization decisions in client
state. Require step-up authentication or confirmation for owner transfer,
credential rotation, domain removal, retention, export, and destructive
operations. Show safe audit history and make security-relevant changes
observable.

Admin pages must disclose stale data and pagination, avoid fetching
unnecessary message content, and provide keyboard/screen-reader accessible
flows. Tenant owners/admins see only the scopes granted by the policy matrix;
platform operators use a separate break-glass path with reason and audit.

## Remaining work and why

Complete durable tenant/user/workspace/domain/API-key/billing/usage/retention/
integration/audit APIs and their authorization first. Define admin role
capabilities, approval and step-up rules, audit views, empty/error states,
account recovery, accessible UX, and support boundaries. Replace
`X-Dev-Subject`/local identity with Phase 74 identity before any production
admin surface.

This remains open because the current UI is a development dashboard over
partial API state, and several write operations are in-memory. A polished UI
without authoritative persistence and server authorization would present
misleading controls and risk losing or overexposing tenant state.

## Acceptance criteria

- Every administrative action maps to a server-side permission and durable
  audit event.
- Cross-tenant and delegated-resource checks are tested independently of UI.
- Sensitive changes use confirmation/step-up/approval where policy requires.
- UI shows request IDs and safe actionable errors without leaking secrets.
- Accessibility, keyboard navigation, stale-state behavior, and responsive
  layouts are tested.

## Research references

- [Phase 73 Organization and Workspace Management](phase-73-organization-workspace-management.md)
- [Phase 74 Authentication and Identity](phase-74-authentication-identity.md)
- [Phase 75 RBAC and Permission Engine](phase-75-rbac-permission-engine.md)
- [Current dashboard](../../smtp-dashboard/src/app/page.tsx)
