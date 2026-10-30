# Frontend import conventions

## Ordering

Group imports consistently in this order, with a blank line between groups:

1. React and Next.js.
2. External dependencies.
3. Application infrastructure (`@/lib/...`).
4. Feature modules (`@/features/...`).
5. Short relative imports for files within the same cohesive feature.
6. Type-only imports, kept explicit with `import type`.
7. Styles, where required.

Follow the installed ESLint and formatter behavior; do not add import-order
rules or tooling that conflict with repository configuration.

## Boundaries

- Use the existing `@/*` TypeScript alias for stable imports under
  `smtp-dashboard/src/`.
- Prefer a feature's public entry point only when one is intentionally
  maintained. Otherwise, import its public module directly; do not reach into
  unrelated feature internals.
- Keep close implementation relationships relative when that is clearer.
- Keep endpoint-specific operations inside their owning feature. Only generic
  HTTP transport belongs in `lib/api/`.
- Do not import server-only modules into client components. The browser API
  client must not import `API_URL` or other server configuration.
- Use `import type` for types that are not needed at runtime.
- Avoid wildcard barrels, duplicate aliases, circular feature dependencies,
  and type assertions that conceal a mismatch with the Go API contract.

The app's current `@/*` alias maps to `src/*` in
`smtp-dashboard/tsconfig.json`; preserve that mapping in application, test,
and lint tooling if those tools are added later.
