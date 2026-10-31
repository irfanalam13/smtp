# Dashboard UI refinement checklist

This checklist captures the UI-only refinement pass. It preserves the existing
Next.js route, client boundary, same-origin API proxy, and feature contracts.

## Shared foundation

- [x] Review current routes, feature services, types, hooks, reusable UI, and Go-backed capabilities.
- [x] Establish a restrained control-plane token layer for color, type, spacing, elevation, focus, and motion.
- [x] Refine the desktop shell, workspace context, compact top bar, navigation states, and mobile drawer.
- [x] Add an accessible reusable confirmation dialog for consequential actions.
- [x] Improve common loading, error, empty, table, status, form, code, and responsive patterns.

## Product sections

- [x] Overview: real readiness, setup, usage, key, domain, and queue context with concise next actions.
- [x] Send email: labelled composition flow, field feedback, duplicate-submission state, and queue-semantics guidance.
- [x] Activity: scannable submission selection, attempts, technical identifiers, diagnostics, and status treatment.
- [x] Domains: registration, DNS challenge copying, readiness, diagnostics, and confirmed disable action.
- [x] Mailboxes: mailbox selector, message list, metadata detail, and inaccessible/empty/loading paths.
- [x] Mailbox administration: management forms, quota, folders, aliases, access, labels, audited message tools, and confirmed destructive operations.
- [x] API keys: least-privilege scope chooser, one-time secret treatment, expiry state, and confirmed revocation.
- [x] SMTP credentials: one-time password treatment, expiry state, rotation, and confirmed revocation.
- [x] Authentication / tenant context: memory-only credentials, workspace context, connection state, and failure feedback.

## Verification

- [x] Baseline lint, typecheck, and frontend tests.
- [x] Final lint, non-incremental typecheck, and 13 frontend tests.
- [x] Webpack production build, including static prerender of `/`.
- [ ] Default Turbopack build: blocked in this environment because its CSS worker cannot bind a local port (`Operation not permitted`).
