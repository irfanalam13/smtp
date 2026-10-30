# Frontend organization migration notes

## Scope and plan

The repository audit found one frontend package, `smtp-dashboard/`, in the
root pnpm workspace. It uses Next.js 16 App Router, React 19, strict TypeScript,
ESLint, and one global stylesheet. Its single `/` route previously contained
the entire interactive dashboard, API request code, feature contracts, data
loading, mutations, and all five view renderers in a 1,225-line client
component.

The refactor follows these stages:

1. Keep the existing App Router, package, root scripts, API rewrite, and `/`
   URL.
2. Move the dashboard's contracts and endpoint operations into its
   `control-plane` feature; isolate generic JSON transport under `lib/api/`.
3. Move dashboard data and action state into a feature hook.
4. Split the overview, compose, domain, mailbox, and API-key screens into
   cohesive feature components; retain shared dashboard-only presentation
   helpers within the feature.
5. Keep route composition server-rendered, make the feature component the
   client boundary, and document actual responsibilities and validation.

## Decisions

- No monorepo layers or shared UI package were added: there is one application
  and no cross-application component reuse.
- No auth, contacts, settings, or notification feature was invented. The
  current token/tenant connection form remains part of the control-plane
  dashboard.
- View selection remains in memory at `/`; the refactor does not create new
  URLs or change routing.
- The global CSS and existing visual design remain in place. There was no
  existing CSS Modules convention or independently reused design system.
- No state library, API client dependency, validation library, or frontend
  test runner was introduced.
- Browser token handling, tenant headers, development identity behavior,
  same-origin API proxying, and Go API contracts remain unchanged.
- Empty shared-component, feature, infrastructure, and test directories are
  tracked with `.gitkeep` markers only. They are organizational scaffolds, not
  implemented features, providers, stores, or tests.

## Follow-up

The application has no automated frontend unit, integration, or E2E test
runner. Adding tests should follow an explicit repository-wide tooling
decision. Additional product capabilities should receive separate feature
boundaries when they become part of the implemented application, not merely
because they appear in a future architecture target.
