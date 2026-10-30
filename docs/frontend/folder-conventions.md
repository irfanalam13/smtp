# Frontend folder conventions

The frontend is the single Next.js application in `smtp-dashboard/`. Keep its
existing root and `src/` layout; do not introduce `apps/` or `packages/` until
the repository has a genuine multi-application or shared-package need.

## Responsibilities

| Location                                               | Responsibility                                                                 |
| ------------------------------------------------------ | ------------------------------------------------------------------------------ |
| `smtp-dashboard/src/app/`                              | App Router special files, route composition, metadata, global CSS, and favicon |
| `smtp-dashboard/src/features/<capability>/components/` | UI owned by that product capability                                            |
| `smtp-dashboard/src/features/<capability>/hooks/`      | Stateful React workflows used by that feature                                  |
| `smtp-dashboard/src/features/<capability>/services/`   | Feature-specific API operations and data access                                |
| `smtp-dashboard/src/features/<capability>/types/`      | Feature-specific API and view-model contracts                                  |
| `smtp-dashboard/src/lib/`                              | Technical infrastructure shared across features                                |
| `smtp-dashboard/src/components/`                       | Shared UI and layout components, when they are actually introduced             |
| `smtp-dashboard/src/hooks/`                            | React hooks shared across multiple features                                    |
| `smtp-dashboard/src/providers/`                        | Application-level React providers, when required                               |
| `smtp-dashboard/src/stores/`                           | Shared client state, only when local feature state is insufficient             |
| `smtp-dashboard/src/types/`                            | Contracts shared across more than one feature                                  |
| `smtp-dashboard/src/constants/`                        | Stable constants shared across features                                        |
| `smtp-dashboard/tests/`                                | Future unit, integration, and E2E tests                                        |
| `smtp-dashboard/public/`                               | Static files addressed by public URL                                           |

`control-plane` is the current feature boundary because domains, queue
submissions, API keys, mailbox metadata, and the account overview are presented
as one authenticated control-plane console. Empty `auth/`, `mail/`, `domains/`,
and `api-keys/` feature folders are tracked scaffolds for capabilities already
present in the product; they are not duplicate implementations. Keep current
code in `control-plane/` until a feature is deliberately extracted. Do not
create folders for unimplemented contacts, billing, notifications, or other
roadmap items.

Empty scaffold directories contain only `.gitkeep` so Git tracks them. Remove
the marker when adding the first real file to that directory. Do not add
placeholder source files or create subdirectories for responsibilities that
do not apply.

## Placement rules

- Keep Next.js files named `page.tsx`, `layout.tsx`, and other framework
  conventions. `app/page.tsx` composes the `/` route; dashboard interaction
  belongs to the feature component.
- Keep feature-owned components, hooks, service calls, and types together.
  Do not put product behavior in generic UI or infrastructure folders.
- Use `lib/api/` for the shared HTTP transport, not endpoint-specific
  operations.
- Keep global styles and design tokens in the existing `app/globals.css`.
  Do not add a second styling system or a shared UI package without reuse
  across multiple applications.
- Keep tests next to their feature or in the test layout established when a
  runner is introduced. There is no configured frontend test runner today.
- Preserve package-level configuration at the application root. The root
  workspace and scripts remain authoritative for invoking dashboard tasks.

## Naming

- Use kebab-case filenames for components, hooks, services, schemas, types,
  and utilities; retain Next.js special filenames exactly.
- Use PascalCase for React component and TypeScript type names.
- Use camelCase for values and functions.
- Name hooks with the `use-` prefix and API modules for their capability.
- Use an `index.ts` public entry point only when it provides a meaningful
  boundary; do not add barrels that re-export implementation internals.

Before creating a folder, identify the code responsibility it will own and
whether an existing feature or infrastructure location already owns it.
