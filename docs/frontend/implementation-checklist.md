# SMTP dashboard implementation checklist

This checklist is based on the working-tree frontend and the Go handlers,
authentication rules, and OpenAPI contracts inspected before implementation.
Existing local changes are preserved. “Complete” means the implementation and
its relevant checks have been verified; unsupported capabilities are not
counted as complete.

## File-by-file checklist

| Status   | File                                                                                     | Work                                                                                                                                                   |
| -------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Complete | `smtp-dashboard/next.config.ts`                                                          | Keep `/api/backend/*` same-origin; use and validate server-only `API_URL`; allow the loopback default only in development.                             |
| Complete | `.env.example`                                                                           | Document the server-side dashboard upstream URL without a `NEXT_PUBLIC_` alias.                                                                        |
| Complete | `smtp-dashboard/src/lib/api/json-client.ts`                                              | Add typed HTTP errors, safe JSON/error parsing, required headers, cancellation, timeout/retry metadata, and relative-path enforcement.                 |
| Complete | `smtp-dashboard/src/features/control-plane/types/control-plane.ts`                       | Align models with actual envelopes, nullable fields, statuses, tenant, readiness, message details, and mutations.                                      |
| Complete | `smtp-dashboard/src/features/control-plane/services/control-plane-api.ts`                | Cover implemented dashboard operations with encoded identifiers and `URLSearchParams`, including tenant, diagnostics/readiness, and mailbox details.   |
| Complete | `smtp-dashboard/src/features/control-plane/hooks/use-control-plane-dashboard.ts`         | Add abort/stale-response protection, per-operation states, scoped key creation, validation, truthful submission feedback, and one-time secret cleanup. |
| Complete | `smtp-dashboard/src/features/control-plane/components/shell/control-plane-dashboard.tsx` | Add accessible responsive menu controls, current tenant/readiness context, and view-level status/retry feedback.                                       |
| Complete | `smtp-dashboard/src/features/control-plane/components/shell/dashboard-sidebar.tsx`       | Implement active navigation, focus-managed mobile drawer, Escape/backdrop close, and disabled future items.                                            |
| Complete | `smtp-dashboard/src/features/control-plane/components/views/overview-view.tsx`           | Show real principal/tenant/readiness/usage/submission data without invented metrics or delivery claims.                                                |
| Complete | `smtp-dashboard/src/features/control-plane/components/views/activity-view.tsx`           | Show actual queue/attempt states, permission errors, and independently retryable submission/attempt loads.                                             |
| Complete | `smtp-dashboard/src/features/control-plane/components/views/api-keys-view.tsx`           | Validate names/scopes/expiry; safely reveal once; show expiry/revocation state and confirmed revoke actions.                                           |
| Complete | `smtp-dashboard/src/features/control-plane/components/views/domains-view.tsx`            | Wire registration, challenge, verification, DNS diagnostics, cached results, and readiness to their actual contracts.                                  |
| Complete | `smtp-dashboard/src/features/control-plane/components/views/mailboxes-view.tsx`          | Show authorized mailboxes, metadata, actual message details, and clear permission/unsupported states.                                                  |
| Complete | `smtp-dashboard/src/features/control-plane/components/views/send-email-view.tsx`         | Validate fields and recipients, retain retry identity, prevent duplicates, and explain queue acceptance accurately.                                    |
| Complete | `smtp-dashboard/src/features/control-plane/components/shared/feedback-states.tsx`        | Provide accessible loading, error, retry, and empty feedback.                                                                                          |
| Complete | `smtp-dashboard/src/features/control-plane/components/shared/submission-table.tsx`       | Keep queue labels accurate and make the table usable at narrow widths.                                                                                 |
| Complete | `smtp-dashboard/src/features/control-plane/components/shared/stat-card.tsx`              | Keep dashboard metrics typed and accessible.                                                                                                           |
| Complete | `smtp-dashboard/src/features/control-plane/utils/formatters.ts`                          | Safely format missing/invalid timestamps and distinguish simulated queue outcomes.                                                                     |
| Complete | `smtp-dashboard/src/features/control-plane/utils/validators.ts`                          | Validate domains and plain-text submissions against the UX-relevant backend limits without mutating input.                                             |
| Complete | `smtp-dashboard/src/app/globals.css`                                                     | Refine responsive shell/drawer, form/table behavior, visible focus, transitions, and reduced-motion handling.                                          |
| Complete | `smtp-dashboard/package.json`                                                            | Add Vitest and DOM testing dependencies plus a dashboard test script.                                                                                  |
| Complete | `pnpm-lock.yaml`                                                                         | Record only the test dependencies required by the manifest change.                                                                                     |
| Complete | `pnpm-workspace.yaml`                                                                    | Explicitly allow esbuild's required binary setup script so frozen installs succeed.                                                                    |
| Complete | `smtp-dashboard/vitest.config.ts`                                                        | Configure the workspace alias and DOM test environment.                                                                                                |
| Complete | `smtp-dashboard/tests/unit/json-client.test.ts`                                          | Verify same-origin transport, headers, path safety, parsing, error mapping, and cancellation.                                                          |
| Complete | `smtp-dashboard/tests/unit/control-plane-api.test.ts`                                    | Verify endpoint paths, methods, query parameters, and mutation payloads.                                                                               |
| Complete | `smtp-dashboard/tests/unit/validators.test.ts`                                           | Verify domain and submission input validation.                                                                                                         |
| Complete | `smtp-dashboard/tests/integration/control-plane-dashboard.test.tsx`                      | Verify mobile drawer semantics, body-scroll lock, Escape close, focus trapping, and restoration.                                                       |
| Complete | `smtp-dashboard/README.md`                                                               | Document local and deployed proxy configuration and transient authentication behavior.                                                                 |
| Complete | `docs/api/README.md`                                                                     | Explain why normal dashboard traffic needs no browser CORS and how to configure the proxy.                                                             |
| Complete | `docs/api/openapi.yaml`                                                                  | Document the existing unauthenticated readiness endpoint used by the dashboard.                                                                        |
| Complete | `docs/configuration.md`                                                                  | State local versus production `API_URL` requirements and the server-side-only boundary.                                                                |
| Complete | `docs/frontend/architecture.md`                                                          | Record the implemented structure, proxy/auth behavior, actual API boundaries, and test commands.                                                       |

## Contract and scope findings

- The Go API already registers the requested health, principal, tenant, domain,
  API-key, mailbox metadata/message, usage, submission, and attempt operations.
- `POST /v1/domains/{id}/checks` returns a `DomainCheck` directly; cached
  diagnostics use `GET /v1/domains/{id}/checks`; domain readiness is a separate
  `GET /v1/domains/{id}/readiness`.
- API-key list/create/revoke require `keys:write`. Creation requires nonempty
  scopes that the caller is allowed to delegate. Submissions require
  `Idempotency-Key`; acceptance does not prove external delivery.
- Mailbox reads require `mailbox:read`, a durable user-bound principal, and
  mailbox ownership or an explicit grant. The API exposes message metadata and
  audited content routes; the dashboard must not render untrusted HTML.
- The browser calls the same-origin `/api/backend/*` rewrite. No browser CORS
  response was reproduced; the configuration defect was the public
  `NEXT_PUBLIC_API_URL` rewrite fallback. The proxy now uses server-only
  `API_URL`; no wildcard CORS or Go middleware was added.
- Vitest unit/integration tests cover transport, API mappings, validation, and
  mobile drawer behavior. A live production API was not available; the rewrite
  was smoke-tested against a local echo upstream.

## Completion status

- [x] Same-origin proxy and environment configuration verified.
- [x] Typed client and service contracts verified against Go handlers.
- [x] Dashboard workflows, access states, and responsive navigation implemented.
- [x] Automated frontend tests and quality gates pass.
- [x] Documentation and final limitations are accurate.

Backend dependencies or unavailable capabilities will be recorded here only
if verification shows that the required endpoint or contract is genuinely
missing.
