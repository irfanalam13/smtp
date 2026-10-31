# SMTP Platform dashboard

The Next.js dashboard provides a browser UI for the Go control-plane API:

- domain registration, TXT verification, and DNS diagnostics;
- API-key creation/revocation and mailbox metadata access;
- plain-text message submission and recent durable queue state.

Run the API and PostgreSQL first, then start the dashboard from the repository
root:

```sh
pnpm dev:dashboard
```

The UI is available at `http://127.0.0.1:3000`. API requests use the same-origin
`/api/backend/*` rewrite, which forwards to `API_URL` (default
`http://127.0.0.1:8080`). Set `API_URL` on the dashboard server when the API
runs elsewhere. Non-development deployments must set it to the API's absolute
HTTP(S) origin (use HTTPS in production). It is server-side configuration,
not a browser secret; the browser talks only to the same-origin rewrite, so
the API does not need wildcard CORS access for this dashboard.

In a development build, the dashboard sends the local development principal
when no bearer token is supplied; the API accepts it only when
`CONTROL_PLANE_DEV_MODE=true` (never enable this in deployment). Otherwise,
paste a user-bound API key or OIDC access token into the connection field.
Tokens are held in memory only and cleared on refresh. `X-Tenant-ID` can
select an active membership for multi-tenant OIDC users.

Submissions require `delivery:write` and are stored in the durable PostgreSQL
queue. Durable accounts must verify the sender domain with its TXT challenge;
the local development principal uses process-local domain state. Usage counters
remain process-local. The current development worker simulates processing and
does not send external email. The dashboard does not claim delivery or mailbox
placement.

Run dashboard quality checks and frontend tests from the repository root:

```sh
pnpm lint:dashboard
pnpm typecheck:dashboard
pnpm --dir smtp-dashboard test
pnpm build:dashboard
```

The dashboard's App Router composition, control-plane feature ownership,
client/server boundaries, and import conventions are documented in
[`docs/frontend/architecture.md`](../docs/frontend/architecture.md),
[`docs/frontend/folder-conventions.md`](../docs/frontend/folder-conventions.md),
and [`docs/frontend/import-conventions.md`](../docs/frontend/import-conventions.md).
