# Frontend architecture

## Current application

The frontend is one private Next.js application in `smtp-dashboard/`, included
in the root pnpm workspace. It uses Next.js 16 App Router, React 19, and strict
TypeScript. The only application route is `/`; dashboard sections are client
side views within that route, not separate URLs. The UI calls the Go API
through the same-origin `/api/backend/*` rewrite in `next.config.ts`.

There are no shared frontend packages, API client package, state library,
provider tree, or test runner today. The dashboard's browser-only token and
tenant selection remain in memory. The Go API remains authoritative for
authentication, authorization, domain verification, and queue state.

## Source layout

```text
smtp-dashboard/
  src/
    app/
      favicon.ico
      globals.css
      layout.tsx
      page.tsx
    components/
      ui/                 # Empty scaffold
      layout/             # Empty scaffold
      navigation/         # Empty scaffold
      feedback/           # Empty scaffold
      forms/              # Empty scaffold
    features/
      control-plane/
        components/
          dashboard-sidebar.tsx
          home-view.tsx
          activity-view.tsx
          api-keys-view.tsx
          control-plane-dashboard.tsx
          dashboard-primitives.tsx
          domains-view.tsx
          mailboxes-view.tsx
          send-email-view.tsx
        hooks/
          use-control-plane-dashboard.ts
        services/
          control-plane-api.ts
        types/
          control-plane.ts
        schemas/           # Empty scaffold
        utils/             # Empty scaffold
      auth/                # Empty scaffold
      mail/                # Empty scaffold
      domains/             # Empty scaffold
      api-keys/            # Empty scaffold
    lib/
      api/
        json-client.ts
    hooks/                 # Empty scaffold
    providers/             # Empty scaffold
    stores/                # Empty scaffold
    types/                 # Empty scaffold
    constants/             # Empty scaffold
  tests/
    unit/                  # Empty scaffold
    integration/           # Empty scaffold
    e2e/                   # Empty scaffold
```

`app/` owns Next.js routing, root metadata/layout, and global styles. The `/`
page is a server component that composes the interactive dashboard. The
`control-plane` feature owns its view state, API operations, types, and
feature-specific UI. `lib/api/` owns only generic JSON transport behavior.
All folders labeled “Empty scaffold” contain only a `.gitkeep` marker so Git
can preserve the future structure. They do not contain placeholder
implementations or indicate that those capabilities or test runners are
already implemented.

The control-plane dashboard navigation is a dedicated sidebar component with
expandable product groups. Existing views remain selectable; future product
areas appear disabled and marked “Soon” until their API and implementation
exist. The menu is an information architecture, not a claim that every
listed capability is available.

## Component and data boundaries

- Server components are the default. The dashboard is a client component
  because it needs event handlers, React state, and browser APIs.
- Keep that client boundary at the dashboard feature; the root layout and
  route composition do not need to become client components.
- Feature views receive typed state and callbacks. They do not construct API
  requests themselves.
- The feature service maps product operations to the existing Go API routes.
  The shared JSON client handles headers, same-origin fetch, response parsing,
  and API error messages.
- Do not move privileged API credentials into browser code. `API_URL` remains
  server-side Next.js configuration; user tokens remain transient browser
  input and are never persisted by the dashboard.
- Domain authorization and sender verification are server responsibilities.
  Frontend state and disabled controls are not authorization boundaries.

## State and routing

The dashboard uses local feature state rather than a global store: its current
views are colocated in one console and do not require cross-route persistence.
The current browser token, optional tenant ID, compose draft, and one-time API
key secret are held in memory. Refreshing the page clears these values.

The existing view selector is intentionally not converted into routes or route
groups. This keeps `/` and existing links stable and avoids adding routing
behavior the product does not currently have.

## Styling and assets

`app/globals.css` remains the single stylesheet because the existing dashboard
uses one integrated visual system and has no CSS Modules or shared UI package.
Feature-specific class names stay close to the feature components; global
tokens, resets, responsive rules, and shared layout styles remain in the
stylesheet. Existing public assets and the app favicon are retained.

## Adding a route or capability

Add a Next.js special file under `app/` only when a new URL is required. Keep
route files focused on composing the route's server/client components. For a
new control-plane capability, add its API operation and contract type under
`features/control-plane/`, then keep its UI with that feature. Introduce a new
feature folder only when it represents a distinct existing product capability
with its own ownership and dependencies. Put a helper in `lib/` only when it
is technical infrastructure shared beyond one feature.

## Validation and tests

Use the root pnpm dashboard scripts or run the equivalent from
`smtp-dashboard/`:

```sh
pnpm lint:dashboard
pnpm typecheck:dashboard
pnpm build:dashboard
```

The repository does not currently configure a frontend unit, integration, or
browser-test runner. Do not add a competing test stack as part of routine
organization; establish one when the project adopts a testing strategy.
