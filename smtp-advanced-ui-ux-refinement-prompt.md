# SMTP Dashboard — Advanced UI/UX Refinement & Product-Grade Design Prompt

## Mission

Act as a principal product designer, senior frontend architect, and design-systems engineer. Upgrade the entire existing SMTP Email Cloud Platform frontend into a polished, distinctive, enterprise-grade product interface.

Improve every page, section, navigation item, panel, card, table, form, dialog, dropdown, tooltip, chart, empty state, loading state, and responsive layout. The result must feel like a carefully art-directed software product built by an experienced product team—not a generic AI-generated SaaS dashboard.

This is a visual and interaction refinement task, not permission to replace the product architecture or invent backend capabilities. Preserve working functionality and integrate with the real backend.

## 1. Inspect Before Changing Anything

- Read the frontend implementation specification and existing frontend architecture documentation.
- Inspect the complete `smtp-dashboard/` source tree, every route, feature, component, stylesheet, asset, and existing design token.
- Review the actual backend API contracts and current frontend API services before changing data-driven UI.
- Identify every existing page, navigation destination, feature state, reusable component, and unfinished or disabled capability.
- Run current lint, typecheck, and build commands to establish a baseline.
- Create a UI refinement checklist grouped by page, component, and shared design-system work.
- Do not assume a feature exists because it appears in navigation or documentation. Verify its implementation and API support.

## 2. Establish a Distinctive Visual Direction

Create a coherent visual identity appropriate for a serious email infrastructure and cloud platform.

The interface should communicate reliability, precision, security, operational clarity, and technical maturity. Provide a clear information hierarchy for technical operators and business users. Use high information density where operational data requires it, without visual clutter.

Avoid generic AI-generated dashboard patterns:
- No excessive gradients, glowing borders, glassmorphism everywhere, neon accents, or decorative blobs.
- No random floating shapes, meaningless illustrations, or decorative charts with fabricated data.
- No excessive rounded cards, pill buttons, badges, and nested containers.
- No repetitive card grids where a well-designed table, list, split panel, or compact metric row is more appropriate.
- No giant headings, oversized empty hero sections, or marketing-site layouts inside operational screens.
- No emoji as primary product icons.
- No arbitrary animations or effects that reduce clarity.
- No copied brand identity or pixel-perfect imitation of another product.

Use restraint. Every color, border, shadow, icon, animation, and spacing choice must serve a purpose.

## 3. Build a Consistent Design System

Create or refine reusable design tokens for:
- Color roles: application background, elevated surfaces, borders, primary text, secondary text, muted text, primary action, focus, success, warning, danger, and informational states.
- Typography: font family, scale, weight, line height, letter spacing, numeric alignment, and readable data labels.
- Spacing: consistent layout and component spacing scale.
- Shape: restrained radius scale, border widths, and separators.
- Elevation: minimal, consistent shadows only where layering requires them.
- Layout: page width, content gutters, sidebar width, topbar height, and responsive breakpoints.
- Motion: duration, easing, distance, and reduced-motion behavior.
- Layering: menus, popovers, dialogs, drawers, toasts, and overlays.

Create accessible reusable primitives where justified:
- Buttons, icon buttons, inputs, textareas, selects, checkboxes, radios, switches, and search fields.
- Tabs, segmented controls, breadcrumbs, pagination, and navigation items.
- Dropdown menus, tooltips, popovers, dialogs, confirmation dialogs, and mobile drawers.
- Tables, data lists, status indicators, badges, metric displays, and empty states.
- Skeletons, spinners, progress indicators, alerts, toasts, and inline errors.
- Page headers, section headers, filter bars, toolbars, and responsive content containers.

Do not create abstractions for one-off elements without a clear reuse case. Keep components typed, focused, composable, and maintainable.

## 4. Upgrade the Global Application Shell

### Desktop
- Design a balanced navigation sidebar and top application bar.
- Make product identity, workspace/tenant context, active navigation, and user controls immediately understandable.
- Provide clear active, hover, focus, expanded, collapsed, and disabled navigation states.
- Group navigation by meaningful product areas and keep labels concise.
- Make the content area use available screen space without becoming excessively wide.
- Align page titles, breadcrumbs, primary actions, and contextual actions consistently.

### Mobile and Tablet
- Replace the desktop sidebar with a proper menu button and accessible navigation drawer.
- Support opening, closing, backdrop dismissal, Escape-key dismissal, and focus management.
- Close the drawer after selecting a destination.
- Prevent background scrolling while the drawer is open.
- Ensure menus, dialogs, tables, forms, and toolbars remain usable on narrow screens.
- Avoid horizontal overflow and tiny touch targets.

Preserve existing routes and navigation behavior unless a change is required and verified. Do not show unfinished capabilities as active features. Clearly distinguish unavailable or planned capabilities without suggesting they work.

## 5. Refine Every Existing Product Section

Inspect every existing page and view. Apply these principles wherever relevant, without inventing unsupported features.

### Overview / Home
- Prioritize meaningful operational information and the most important next actions.
- Use real backend data only.
- Show clear time ranges and units for metrics.
- Distinguish queue acceptance, processing, and actual remote delivery accurately.
- Provide useful recent activity and actionable system states.
- Avoid decorative metrics and fabricated trends.

### Domains
- Improve domain registration, domain lists, verification steps, DNS diagnostics, and readiness presentation.
- Make verification status and required DNS records easy to understand and copy.
- Explain actionable failures in plain language while retaining technical details.
- Clearly separate pending, checking, verified, degraded, and failed states based on actual API responses.
- Use confirmation and feedback for important actions.

### Mailboxes
- Improve mailbox selection, mailbox metadata, message lists, search/filter controls, and message detail presentation where supported.
- Make sender, recipient, subject, timestamp, and message state scannable.
- Handle empty, inaccessible, loading, and error states distinctly.
- Do not imply mailbox mutation, folders, attachment previews, or message actions are available unless the backend supports them.

### Activity / Submissions
- Design operational tables and detail panels for submissions, queue states, and attempts.
- Distinguish queued, processing, simulated, delivered, deferred, and failed states only when the API actually supplies those meanings.
- Never label queue acceptance or simulated processing as successful external email delivery.
- Provide useful filtering, sorting, pagination, timestamps, and request/error details where supported.
- Make long identifiers and technical diagnostics readable and copyable.

### API Keys
- Make key creation, scope selection, expiry, listing, and revocation clear and safe.
- Explain that secrets are shown once where applicable.
- Display a one-time secret with a deliberate copy interaction and clear warning.
- Never display or persist a secret after the backend's one-time response.
- Require confirmation for revocation and clearly communicate the result.
- Never invent scopes or permissions; use the actual backend contract.

### Send Email / Compose
- Improve sender, recipient, subject, and message composition ergonomics.
- Add clear validation and field-level error messages.
- Support multiple recipients only as permitted by the API.
- Prevent accidental duplicate submission and use the backend's idempotency contract correctly.
- Clearly distinguish accepted into queue from delivered.
- Preserve drafts only if the existing product's security and persistence design supports it.

### Authentication / Tenant Context
- Make authentication and tenant selection states understandable.
- Handle missing credentials, expired credentials, unauthorized access, and unavailable tenant context gracefully.
- Never expose privileged credentials or introduce insecure browser token persistence.
- Do not create a fake login or session experience when backend authentication does not support it.

### Other Existing Sections
Review every remaining implemented section individually. Improve its layout, hierarchy, controls, responsiveness, feedback, and accessibility according to its actual purpose and API contract. Do not leave a section visually inconsistent simply because it is less frequently used.

## 6. Make Every Component Feel Finished

Review components individually, including:
- Buttons: hierarchy, size, loading, disabled, destructive, icon-only, and focus states.
- Forms: labels, helper text, required indicators, validation, error placement, and submission feedback.
- Tables: column sizing, alignment, row density, hover, selection (only if functional), responsive behavior, and empty states.
- Cards and panels: meaningful grouping, consistent padding, restrained borders, and clear headings.
- Dropdowns and menus: placement, keyboard navigation, dismissal, and layering.
- Dialogs: clear title, concise explanation, safe action order, focus handling, and mobile sizing.
- Tooltips: concise explanations and correct delay/placement.
- Badges: only for meaningful status or classification; never as decoration.
- Icons: consistent icon family, size, stroke, and semantic meaning.
- Charts: real data, readable axes, labels, units, useful empty states, and accessible alternatives.
- Toasts and alerts: clear message, appropriate severity, and non-intrusive placement.
- Loading states: stable skeletons that match final content geometry.
- Empty states: explain why there is no data and provide a relevant next action.
- Error states: explain what failed, preserve user input where safe, and offer retry only when retry is valid.

## 7. Motion, Hover, and Micro-Interactions

Add purposeful, subtle motion to improve feedback:
- Smooth, restrained hover transitions for interactive elements.
- Clear pressed, focused, selected, expanded, and disabled states.
- Subtle navigation and drawer transitions.
- Dialog and popover entrance/exit transitions.
- Small feedback transitions for successful copy, save, and state changes.
- Skeleton shimmer only if subtle and non-distracting.
- Avoid animating large areas or every element on page load.
- Never make hover the only way to reveal essential information.
- Support `prefers-reduced-motion`.
- Use CSS transitions or existing approved dependencies. Do not add a large animation library without a demonstrated need.

## 8. Responsive Design Requirements

Test representative widths:
- Small mobile: 320–375px.
- Standard mobile: 390–430px.
- Tablet: 768px.
- Laptop: 1024–1280px.
- Desktop: 1440px and wider.

At every width:
- No accidental horizontal scrolling.
- No clipped menus, dialogs, popovers, or content.
- No overlapping navigation or page actions.
- Tables must use an intentional strategy: controlled horizontal scrolling, prioritized columns, or a suitable compact representation.
- Forms and primary actions must remain usable by touch.
- Content must retain readable line lengths and hierarchy.

## 9. Accessibility and Interaction Quality

Meet practical WCAG 2.2 AA expectations:
- Semantic HTML and correct heading hierarchy.
- Keyboard access to all interactive controls.
- Visible focus indicators.
- Accessible names for icon-only controls.
- Correct labels and descriptions for form fields.
- Sufficient text and non-text contrast.
- Focus trapping and restoration for modal dialogs.
- Escape and outside-click behavior where appropriate.
- Screen-reader announcements for asynchronous success and error feedback.
- Do not communicate state through color alone.
- Respect reduced-motion preferences.
- Ensure touch targets are appropriately sized.

## 10. Real Data and Backend Integration Must Remain Correct

- Preserve the existing shared API client, feature service boundaries, typed API contracts, and same-origin `/api/backend/*` proxy unless a verified architectural issue requires a change.
- Never call backend endpoints directly from browser code using a hardcoded internal API origin.
- Do not add CORS workarounds such as `mode: "no-cors"`, wildcard origins with credentials, or disabled security.
- Keep authorization and tenant isolation enforced by the backend.
- Do not fabricate records, chart values, statuses, or activity.
- Do not silently swallow API errors.
- Keep loading, success, empty, forbidden, and failure states distinct.
- Use optimistic updates only when rollback and consistency behavior are correctly implemented.
- Preserve idempotency behavior for message submission.
- Ensure UI terminology matches actual backend semantics and documented limitations.

If a visual improvement requires a backend capability that does not exist, do not fake it. Keep the UI honest, document the dependency, and avoid presenting the control as operational.

## 11. Performance and Code Quality

- Preserve Next.js App Router and server/client component boundaries.
- Keep components small and focused; avoid turning the entire application into a client component.
- Avoid unnecessary global state, excessive effects, and redundant requests.
- Use stable keys and memoization only where profiling or clear complexity justifies it.
- Avoid layout shifts and oversized assets.
- Optimize icons, images, and fonts using existing project conventions.
- Keep animations GPU-friendly and minimal.
- Avoid dependency additions unless necessary.
- Maintain strict TypeScript; do not solve type errors with `any`, unsafe casts, or disabled checks.
- Remove dead styles and duplicate components only after confirming they are unused.

## 12. Implementation Workflow — One Section and File at a Time

Work in controlled, reviewable steps:

1. Record the current baseline and inspect all relevant files.
2. Define the visual direction and design tokens.
3. Refine the global shell and shared primitives.
4. Refine one product section at a time.
5. Refine each section's child components and all interaction states.
6. Verify API integration and semantics for that section.
7. Verify desktop, tablet, and mobile layouts.
8. Run formatting, lint, typecheck, tests, and build checks.
9. Fix regressions before moving to the next section.
10. Update the checklist and continue until all existing sections are covered.

Before editing a file, understand its imports, consumers, styles, and responsibilities. Avoid broad rewrites that make regressions difficult to isolate. Preserve working behavior unless an intentional improvement is verified.

## 13. Required Validation

Run the repository's actual scripts, including applicable commands:

```bash
pnpm lint:dashboard
pnpm typecheck:dashboard
pnpm build:dashboard
```

Also:
- Run the existing test suite if configured.
- Add or update component and interaction tests using the project's established test stack.
- Verify keyboard navigation and mobile menu behavior.
- Verify forms, validation, loading, error, empty, and success states.
- Verify real API calls for each supported feature.
- Inspect browser console and network failures.
- Check for horizontal overflow and responsive layout regressions.
- Confirm no CORS errors remain in the supported deployment configuration; if an environment-level issue cannot be fixed in frontend code, identify the exact backend/proxy configuration required.

Do not claim a visual or functional test was performed unless it was actually run.

## 14. Completion Criteria

The task is complete only when:
- Every existing page and section has been reviewed and refined.
- Shared components follow one coherent design system.
- Desktop navigation and mobile menu work correctly.
- Hover, focus, active, disabled, loading, and error states are polished and consistent.
- Every supported UI operation remains connected to its real backend endpoint.
- No fake data or misleading delivery claims have been introduced.
- Responsive layouts work across the required viewport sizes.
- Accessibility and reduced-motion behavior have been addressed.
- Existing functionality has not regressed.
- Formatting, linting, typechecking, tests, and production build pass, or pre-existing/environmental blockers are explicitly documented.

## 15. Final Report

At completion, provide:
- Sections and components refined.
- Design-system tokens and shared primitives added or updated.
- Navigation and responsive improvements.
- Interaction and animation improvements.
- Accessibility improvements.
- Backend-connected features verified.
- Files changed and their purpose.
- Commands and tests actually executed, with results.
- Remaining limitations and any backend work required.

**Final instruction:** Prioritize product quality over visual novelty. Make the SMTP dashboard feel intentional, calm, premium, technically credible, and genuinely usable. Every page should look designed for its actual workflow—not generated from a generic dashboard template. Do not stop after producing a plan; implement, verify, and refine the actual frontend.
