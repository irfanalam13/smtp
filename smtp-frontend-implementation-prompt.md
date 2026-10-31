# SMTP Platform — Complete Frontend Implementation Prompt

## Mission

Act as a senior frontend architect and product UI engineer. Implement the SMTP Email Cloud Platform frontend in the existing repository, integrate it correctly with the existing Go control-plane backend, eliminate browser CORS failures, and deliver a polished, responsive, accessible, production-quality dashboard. **Do the implementation in the repository; do not stop at analysis, plans, mockups, or documentation.**

## 1. Mandatory repository reconnaissance

Before editing anything, inspect the complete repository and current Git state. Read at minimum:

- `docs/frontend/architecture.md`
- `docs/api/README.md` and `docs/api/openapi.yaml`
- `docs/phase-status.md` and relevant frontend/backend implementation notes
- `smtp-dashboard/package.json`, root `package.json`, `pnpm-workspace.yaml`, `smtp-dashboard/next.config.ts`
- All files under `smtp-dashboard/src/app/`, `components/`, `features/control-plane/`, and `lib/api/`
- The Go route registration, handler request/response contracts, authentication middleware, and relevant tests under `smtp-api/`

Treat the checked-in code and OpenAPI contract as the source of truth. Verify every endpoint, HTTP method, payload, response shape, permission, and known limitation against the actual backend. Do not infer that a navigation item or planned feature already has a working API. Preserve existing working behavior and architecture unless a concrete defect requires a change.

Current documented frontend baseline: Next.js 16 App Router, React 19, strict TypeScript, pnpm workspace, a single `/` dashboard route, control-plane feature components/services/hooks/types, a shared JSON client, and a same-origin `/api/backend/*` rewrite. Existing documented views include home, activity, API keys, domains, mailboxes, and send email. The frontend currently has no configured unit, integration, or browser test runner; inspect current package scripts before choosing or adding one.

## 2. Architecture and code organization

Keep the existing Next.js App Router and feature-oriented architecture. Do not rewrite the application into another framework or introduce unnecessary state-management libraries.

Use clear ownership boundaries:

- `src/app/`: route composition, root layout, metadata, global styles.
- `src/components/ui/`: reusable accessible primitives (buttons, inputs, dialogs, dropdowns, badges, cards, tables, skeletons, tooltips, etc.) only where shared.
- `src/components/layout/`: app shell, header/topbar, responsive navigation shell, page container.
- `src/components/navigation/`: desktop sidebar, mobile navigation drawer, navigation groups and active states.
- `src/components/feedback/`: toast/alert, empty, error, loading and confirmation states.
- `src/components/forms/`: reusable form controls and validation wrappers.
- `src/features/control-plane/`: control-plane views, feature hooks, services, schemas, types and utilities.
- `src/lib/api/`: shared transport, typed API errors, request helpers.
- `src/lib/`, `src/hooks/`, `src/providers/`, `src/types/`, `src/constants/`: only genuinely shared infrastructure.

Keep components small, cohesive, typed, and readable. Avoid giant page components, duplicated API calls, circular imports, `any`, unsafe type assertions, duplicated design tokens, and unnecessary abstractions. Keep server components as the default; limit client components to interactive boundaries. Do not move privileged credentials into browser code. Do not store tokens or secrets in localStorage, sessionStorage, cookies created by frontend JavaScript, URL parameters, logs, analytics, or persisted client state.

If the actual repository has evolved from this documented layout, adapt to the real structure and update the architecture documentation rather than forcing an outdated tree.

## 3. Backend integration — connect every implemented API

Create or complete one typed API integration layer. Feature components must call feature hooks/services; they must never construct raw fetch requests or endpoint URLs directly.

Use `docs/api/openapi.yaml` and actual Go handlers as the contract. Audit and integrate every backend endpoint that is implemented and intended for the dashboard, including the following documented operations where confirmed present:

- Readiness: `GET /health/ready`
- Current principal: `GET /v1/me`
- Current tenant: `GET /v1/tenants/current`
- Domains: `GET/POST /v1/domains`
- Domain verification: `POST /v1/domains/{id}/verification`, `POST /v1/domains/{id}/verification/check`
- Domain diagnostics/readiness: `POST /v1/domains/{id}/checks`, `GET /v1/domains/{id}/readiness`
- API keys: `GET/POST /v1/api-keys`, `POST /v1/api-keys/{id}/revoke`
- Mailboxes: `GET /v1/mailboxes`, `GET /v1/mailboxes/{mailbox_id}/messages`, `GET /v1/mailboxes/{mailbox_id}/messages/{message_id}`
- Usage: `GET /v1/usage`
- Submissions: `GET/POST /v1/submissions`, `GET /v1/submissions/{id}/attempts`

This list is a verification checklist, not permission to assume all operations work. Confirm exact response envelopes, query parameters, pagination limits, required headers, and auth scopes in the live code and OpenAPI file. Add missing frontend service methods and typed models for confirmed endpoints. Use URL encoding for path identifiers and `URLSearchParams` for query parameters. Keep response types aligned with actual JSON, including `{ items: [...] }` or nested envelopes where applicable.

For every screen, wire real data and mutations end-to-end:

- Home/overview: current principal and tenant, readiness, usage, recent submission/activity data that the backend actually provides.
- Domains: list, add/register, generate verification challenge, display DNS instructions/records, check verification, run diagnostics, and display readiness results.
- API keys: list safe metadata, create a key with validated name/scopes/expiry if supported, reveal the returned secret exactly once with an explicit copy action, warn that it cannot be retrieved again, and revoke with confirmation. Never show a fake secret or persist the one-time secret.
- Mailboxes: list only mailboxes returned to the authenticated principal; load messages and message details using the actual available API. Show a clear permission/unsupported state where a backend capability is absent.
- Activity/submissions: list submissions and attempts; distinguish queued, processing, failed, and other actual backend states. Do not label queue acceptance or simulated success as successful external email delivery.
- Send email: validate sender, recipients, subject, and body; submit with a unique `Idempotency-Key`; prevent accidental duplicate submissions; show the API's actual acceptance response and clearly explain that queue acceptance is not proof of remote delivery.

Use actual backend permission errors to render helpful access states. The UI is not an authorization boundary; never bypass or weaken backend tenant isolation. Do not fabricate records, charts, counters, delivery statuses, mailbox actions, folders, inbox capabilities, billing, analytics, or settings when the backend does not expose them. Keep unavailable capabilities visibly marked as planned/disabled, or omit them until implemented.

## 4. Fix CORS errors correctly — no unsafe workaround

The browser should communicate with the Go API through the existing same-origin Next.js proxy path `/api/backend/*`. Preserve and verify this architecture rather than making browser requests directly to a different API origin.

1. Inspect `smtp-dashboard/next.config.ts` and the API client. Ensure the rewrite maps `/api/backend/:path*` to the server-side `API_URL` origin and preserves the path, query string, methods, request bodies, and response status/headers as required by Next.js.
2. Use **only server-side `API_URL`** for the upstream origin. Remove the `NEXT_PUBLIC_API_URL` fallback so the private upstream origin is not configured through a public-prefixed variable. Keep the upstream URL out of client bundles and browser code.
3. All browser requests must use relative URLs under `/api/backend`; no hard-coded `http://localhost:8080`, `127.0.0.1`, or production API host in client components.
4. Ensure local, preview, staging, and production environments document the correct `API_URL`, with a safe local development default only where appropriate. Validate missing/malformed configuration and provide a useful server-side error without leaking secrets.
5. Verify that the proxy works for GET and POST, JSON bodies, `Authorization`, `X-Tenant-ID`, `Idempotency-Key`, query parameters, non-2xx responses, and request IDs. Do not strip required headers or silently swallow upstream failures.
6. If any supported deployment genuinely requires direct cross-origin browser access, implement CORS in the Go backend with an explicit environment-configured allowlist of exact trusted frontend origins, required methods and headers, correct OPTIONS preflight handling, and `Vary: Origin`. Never use wildcard `Access-Control-Allow-Origin: *` with credentials, never reflect arbitrary origins, and never treat CORS as authentication. Prefer the same-origin proxy so browser CORS is not needed for the normal dashboard path.
7. Diagnose the actual failing request in browser Network tools and server logs. Distinguish CORS from DNS, TLS, wrong API URL, proxy rewrite, authentication, authorization, and backend availability errors. Do not “fix” CORS by disabling browser security, adding a public proxy, or hiding failed requests.

Acceptance: the dashboard makes no direct cross-origin API calls in its normal flow; same-origin API requests work in development and production configuration; preflight is correct if and only if direct cross-origin access is intentionally supported.

## 5. Shared API client, auth and error handling

Build a robust typed JSON client that:

- Uses relative `/api/backend` URLs and `cache: "no-store"` for authenticated dynamic data.
- Adds `Accept`, JSON `Content-Type` only when appropriate, `Authorization: Bearer ...` when a token exists, and `X-Tenant-ID` only when selected and required.
- Supports `Idempotency-Key`, `AbortSignal`, query strings, and caller-provided headers without accidentally overwriting required headers.
- Parses empty, JSON, and non-JSON error responses safely.
- Exposes a typed error containing HTTP status, backend error code/message, request ID, and safe retry metadata when available.
- Handles 400 validation, 401 authentication, 403 permission, 404, 409 conflict/idempotency, 413 size, 429 rate limit, 5xx, network failure, timeout, and malformed response states distinctly.
- Does not retry non-idempotent mutations blindly. Only retry safe requests or mutations with a stable idempotency key and an explicitly safe retry policy.
- Supports cancellation on unmount and stale-request protection to prevent older responses overwriting newer selections.
- Never logs bearer tokens, API-key secrets, message bodies, or sensitive headers.

Keep the existing transient in-memory token and tenant selection behavior unless the backend/auth architecture is deliberately upgraded. Make it clear to users that refreshing clears transient credentials if that remains the intended model. Never accept an OIDC ID token where the API requires an access token. Do not invent a browser login flow: if Keycloak/browser session support is not implemented, provide a clear secure credential-entry or configured-auth state consistent with the existing product design and backend contract. Never send `X-Dev-Subject` outside local development.

## 6. UI/UX direction — polished Email Cloud control panel

Create a cohesive, premium SaaS dashboard suitable for a serious email infrastructure product. Prioritize clarity, hierarchy, density, speed, and operational trust over decorative effects.

### App shell and navigation

- Desktop: fixed or sticky left sidebar with SMTP product mark, clear grouped navigation, active-route/view indicator, collapsible groups, tenant/workspace context, and user/account area. Main content must resize correctly when the sidebar collapses.
- Tablet: compact/collapsible navigation with usable content width.
- Mobile: replace the desktop sidebar with a visible menu button in the top bar. Open an accessible slide-in drawer or sheet with backdrop, close button, focus management, Escape-to-close, body-scroll handling, and automatic close after navigation. Never squeeze the desktop sidebar onto a phone.
- Keep the current single-route view model unless a real product requirement and implementation justify URL routing. Preserve browser back/forward expectations if routes are introduced.
- Navigation labels and badges must reflect real implemented capabilities. Disabled future items must not look clickable or pretend to work.

### Visual system

- Establish consistent design tokens for color, typography, spacing, radii, borders, shadows, surfaces, and motion.
- Use a restrained professional palette, strong contrast, clear page titles, concise descriptions, consistent cards, tables, filters, forms, and status indicators.
- Create responsive overview cards, domain tables, API-key management, mailbox/message lists, submission activity, and compose forms with consistent layout patterns.
- Include polished loading skeletons, empty states with useful next actions, error/retry states, success feedback, destructive-action confirmation, and inline form validation.
- Add light/dark theme only if it can be implemented consistently and accessibly; do not create half-finished theme toggles.
- Use existing dependencies/assets first. Do not add large UI or animation libraries without a clear need and compatibility review.

### Motion and hover

- Add refined transitions for navigation, dropdowns, dialogs, cards, buttons, tabs, tooltips, and mobile drawer. Use hover states on pointer devices and clear pressed/focus states on touch/keyboard.
- Prefer subtle opacity, color, border, shadow, and small transform transitions; avoid excessive parallax, bounce, large movement, or animation that slows operational workflows.
- Use CSS transitions or already-installed animation tooling; avoid adding a dependency just for trivial hover effects.
- Respect `prefers-reduced-motion: reduce` and disable/reduce nonessential animation accordingly.
- Do not make hover the only way to reveal actions; all controls must be accessible by keyboard and touch.

### Responsive behavior and accessibility

- Design mobile-first and test narrow phones, large phones, tablets, laptops, and wide desktop layouts.
- Tables must become usable on mobile (responsive columns, row cards, or intentional horizontal scrolling with clear affordance); never cause whole-page horizontal overflow.
- Use semantic landmarks, headings, buttons, labels, descriptions, keyboard navigation, visible focus rings, sufficient contrast, accessible dialogs, and screen-reader announcements for async feedback.
- Ensure forms are usable with keyboard and mobile input; provide field-level errors and preserve user input after recoverable failures.
- Use correct loading/disabled states to prevent duplicate actions.

## 7. Product pages and information architecture

Organize the dashboard around capabilities that exist today. At minimum, make the existing Home, Domains, Mailboxes, Activity/Delivery, API Keys, and Send Email experiences coherent and fully wired where backend support exists. Add sub-navigation or detail panels only when backed by real functionality.

Every page must have:

- A clear page title and short explanation of its purpose.
- A consistent content width and responsive spacing.
- A primary action only when the operation exists and the user is authorized.
- Loading, empty, error, success, and permission states.
- Search/filter/sort/pagination only when the data set and backend support justify it; do not fake server pagination client-side for incomplete datasets.
- Accurate status labels and timestamps; format dates consistently and handle missing values gracefully.

## 8. Forms, validation and safety

Use the repository's existing validation approach if present. Otherwise select a small, compatible validation solution only if it materially improves correctness. Validate on both client and server; frontend validation is for UX, not security.

- Validate email addresses and recipient lists, domain syntax, required values, length limits, API-key names/scopes, and date-time inputs according to backend constraints.
- Do not silently normalize user input in ways that change message meaning.
- Confirm revocation and other destructive actions.
- Display one-time API-key secrets only immediately after successful creation, with copy and dismiss controls; clear the secret from component state when dismissed or the view is left.
- Protect against double submission and preserve idempotency keys across safe retries of the same submission attempt.
- Do not render untrusted email content as raw HTML. Escape/sanitize content and avoid unsafe HTML injection.

## 9. Testing and quality gates

Add meaningful tests using the repository's existing test setup if available. If no frontend test runner exists, introduce a minimal, well-supported test strategy only after inspecting compatibility with Next.js 16, React 19, pnpm, and the workspace. Include:

- API client tests for headers, same-origin paths, JSON parsing, error mapping, aborts, and idempotency headers.
- Service contract tests for endpoint paths, methods, payloads, and response types.
- Component tests for responsive navigation, mobile drawer keyboard behavior, forms, empty/loading/error states, and destructive confirmations.
- Integration tests for domain registration/verification, API-key lifecycle, mailbox loading, submission flow, and activity/attempt display using controlled test responses or a test backend.
- End-to-end smoke coverage for desktop and mobile dashboard navigation and the critical API flows when a browser test runner is available.

Run and fix all applicable repository checks, including:

```sh
pnpm lint:dashboard
pnpm typecheck:dashboard
pnpm build:dashboard
```

Also run relevant root workspace checks and new frontend tests. Do not suppress lint/type errors, weaken TypeScript, skip tests, or claim success when a command was not run. Verify production build behavior and ensure the upstream API origin is not exposed in client bundles.

## 10. Documentation and implementation tracking

Update `docs/frontend/architecture.md` to reflect the real final structure, component boundaries, API integration, auth behavior, navigation behavior, and test commands. Update relevant API/CORS deployment documentation with exact environment variables and local/prod proxy behavior. Record any backend endpoint that is genuinely missing as a backend dependency; do not fake it in the frontend.

Maintain a concise implementation checklist with completed, partially completed, and blocked items. For each blocked item, identify the exact backend endpoint/contract or infrastructure dependency and the file(s) involved.

## 11. Execution order

Implement in this order, making real code changes at every stage:

1. Audit repository, backend contracts, frontend baseline, scripts, and current defects.
2. Fix and verify same-origin API proxy configuration and CORS root cause.
3. Complete typed API client, request context, errors, cancellation, and auth/tenant headers.
4. Complete typed service methods for every confirmed implemented API endpoint.
5. Refactor the app shell, desktop sidebar, responsive mobile menu, and shared UI primitives.
6. Implement and wire each existing product view to real backend data and mutations.
7. Add polished forms, validation, status feedback, empty/loading/error states, and safe confirmations.
8. Add responsive layouts, hover/focus/motion behavior, accessibility, and reduced-motion support.
9. Add tests, run all quality gates, fix regressions, and verify production configuration.
10. Update documentation and report exactly what is complete, incomplete, or blocked.

## 12. Non-negotiable rules

- Do not stop after producing a plan. Continue implementing until the frontend scope is complete or a real external dependency blocks it.
- Do not replace real backend integration with mock data, static arrays, fake charts, fake metrics, or hard-coded success responses.
- Do not invent endpoints or assume unsupported backend capabilities. If an endpoint is missing, document the dependency and keep the UI honest.
- Do not use wildcard CORS, disable browser security, expose private API origins/tokens, or bypass backend authorization.
- Do not break the existing pnpm workspace, Next.js App Router, or working backend behavior.
- Do not add unnecessary dependencies, global state, or routing complexity.
- Do not use hover-only interactions; support touch, keyboard, focus, and reduced motion.
- Do not mark an operation successful until the backend confirms it.
- Keep code production-oriented: strict types, accessible components, secure handling of secrets, clear boundaries, and maintainable files.

## Definition of done

The frontend is complete only when the existing product views are coherently organized and responsive; desktop navigation and mobile menu both work; motion/hover/focus behavior is polished and accessible; every implemented backend API intended for the dashboard is connected through typed services; all normal browser API traffic is same-origin through the Next.js proxy; the CORS root cause is verified rather than hidden; auth, tenant scoping, errors, loading, empty, and mutation states are handled; unsupported features are clearly identified rather than simulated; documentation is current; and lint, typecheck, build, and relevant tests pass.
