# API and communication framework

The current control-plane API is a Go `net/http` service using versioned
`/v1` routes. It deliberately does not introduce a second HTTP framework, RPC
transport, gateway, or service mesh.

## Conventions

- JSON request and response bodies use `application/json`.
- Protected routes authenticate through the Phase 07 principal adapter and
  authorize explicit tenant-scoped permissions.
- Every response includes a bounded `X-Request-ID`; a valid client value is
  preserved, otherwise one is generated.
- Errors use `{ "error": { "code", "message", "request_id" } }`.
- List endpoints use `limit`, defaulting to 50 and bounded to 100. Invalid
  values are rejected rather than silently replaced.
- JSON request bodies reject unknown fields and trailing JSON values.
- `/v1` is the current version. Additive response fields are compatible;
  breaking changes require a new version or documented migration.
- API-key creation is a one-time secret response. Secrets are never included
  in error responses or list results.

The machine-readable contract is [openapi.yaml](openapi.yaml). The API accepts
PostgreSQL-backed API keys and, when configured, OIDC bearer access tokens for
pre-provisioned users. OIDC issuer, signature, expiration, issued-at, and
dedicated API audience are verified; current user status, tenant membership,
and permissions come from PostgreSQL. `X-Tenant-ID` selects an active
membership when a user belongs to multiple tenants; it does not grant
membership. Mailbox list/message reads additionally check user-bound mailbox
permissions.

The API also provides generic OIDC Authorization Code login with PKCE and
server-managed browser sessions. Session credentials are opaque, stored only
as database hashes, and delivered in HttpOnly cookies; state, nonce, verifier,
and tenant selection are held in short-lived HttpOnly cookies. Sessions have
idle and absolute expirations, can be listed/revoked per device or logged out
globally, and use CSRF-token plus same-origin checks for unsafe requests.
Identity resolution still requires exact issuer/subject provisioning and an
active membership. The `/v1/auth/sessions` token-exchange route accepts only a
verified OIDC API access token; an ID token is not accepted as an API bearer
token.

Email/password signup creates a new workspace with the registering user as
owner. Password login requires a verified email and creates the same
server-managed browser session as OIDC. Verification and password-recovery
tokens are single-use, stored as hashes, and time-limited; password reset
revokes the user's active sessions. These flows require the configured
authentication SMTP relay and migration `000029`.

This is not a complete identity platform: OIDC user provisioning,
JWT refresh-token lifecycle, GitHub's OAuth identity API, per-tenant provider
management, MFA/passkeys, invitations/account linking, and distributed
authentication rate limits remain unimplemented. SAML and durable domain
control-plane state also remain unimplemented.

To enable OIDC API-token verification, set both `OIDC_ISSUER_URL` and
`OIDC_API_AUDIENCE`. Configure the provider to issue an API access token whose
`aud` contains the dedicated API audience and whose `typ` claim is `Bearer`;
the browser client's ID token is rejected. For browser sign-in, also configure
`OIDC_CLIENT_ID` and `OIDC_CLIENT_SECRET`; the callback must be exactly
`<APP_PUBLIC_ORIGIN>/api/backend/v1/auth/oidc/callback`. OIDC users must already have a row in `users` with the exact issuer and subject
and an active `tenant_memberships` row. Migrations `000015`, `000028`, and
`000029` add identity issuer, browser-session, and password-account storage,
respectively. Do not link identities by email.

Mailbox reads require a durable user identity plus mailbox permission. This
can be a user-bound API key or a configured OIDC user whose membership is
resolved from PostgreSQL. To provision a machine key for an existing active
tenant member:

```sh
set -a
. ./.env
set +a
cd smtp-api
go run ./cmd/bootstrap-key \
  --tenant-id <existing-tenant-uuid> \
  --user-id <existing-member-uuid> \
  --name mailbox-reader \
  --scopes mailbox:read
```

The command prints the secret once; transfer it directly to the approved
secret manager and do not save it in shell history or source control. Later
keys can be issued through `POST /v1/api-keys` by an authenticated key with
`keys:write`, and are persisted in PostgreSQL. Existing tenant-only keys remain
valid for their existing scopes while their tenant is active, but cannot read
mailboxes or create user-bound keys. To enable mailbox access, provision a
separate user-bound key and revoke the legacy key only when it is no longer
needed. User-bound keys stop authenticating if their owner is disabled or
removed from the tenant; expired and revoked keys are rejected. No API key,
including an owner-bound key, receives tenant-wide mailbox access: mailbox
reads also require ownership or an explicit mailbox permission. `X-Dev-Subject`
is not accepted for PostgreSQL mailbox reads.

## SMTP credentials

Tenant owners/admins can grant `smtp:credentials:read` for listing and
`smtp:credentials:write` for create/rotate/revoke operations to a user-bound
API key. Manage submission credentials through `GET`/`POST
/v1/smtp-credentials`, `POST /v1/smtp-credentials/{id}/rotate`, and
`POST /v1/smtp-credentials/{id}/revoke`. A generated password is returned
only at create/rotation and is never included in list responses. Expiry
defaults to 90 days and may be set up to one year. Management changes are
written to the tenant audit table in the same PostgreSQL transaction.

Use the returned `authcid` (the tenant UUID, a colon, and the username) as the
SMTP AUTH PLAIN username. Transfer the one-time password directly into the
approved secret manager; do not log it or store it in source control. SMTP
authentication uses PostgreSQL-backed per-IP, per-tenant, and per-account
throttling shared across ingress instances. See the [OpenAPI contract](openapi.yaml).

Migration `000013` records whether a key is tenant- or user-bound. Apply
migrations `000012` and `000013` in order, with API key issuance paused, before
deploying code that reads `principal_type`. The backfill classifies rows with a
creator as user-bound and rows without one as tenant-bound. If `000012` was
already deployed while user-bound keys could be issued, first audit any
creator-less keys: the old nullable creator column cannot distinguish an
original tenant key from a user key whose membership was removed before
`000013`. After applying `000013` but before restarting the API, classify
ambiguous or known-detached keys as `user` (which fails closed because they
have no active user binding), for example:

```sql
UPDATE api_keys
SET principal_type = 'user'
WHERE id = '<reviewed-key-uuid>' AND created_by_user_id IS NULL;
```

Only leave a key classified as `tenant` when its tenant-only origin is
confirmed. If the origin cannot be confirmed, fail closed and provision a
replacement user-bound key. After `000013`, a user-bound key whose membership
is removed remains classified as user-bound and is rejected; it does not fall
back to tenant access.

Mailbox provisioning, lifecycle suspension/tombstoning, alias and permission
administration, folder operations, append/copy/move, flags, labels, quota
management, and bounded message-content reads are routed to the Rust mailbox
service. Message append requires an `Idempotency-Key` and returns the original
append result for an identical retry; using the key for different message
bytes, folder, or flags returns `409`. Content/header/body reads are audited.
Mailbox read access requires `mailbox:read`; writes and administration require
`mailbox:write` and `mailbox:manage` respectively, and mailbox-level owner or
grant checks are performed in PostgreSQL. Usage counters and DNS diagnostic
cache remain process-local. Mailbox bytes remain inline PostgreSQL `BYTEA`,
and this service slice does not imply production readiness.

## Message submission

`POST /v1/submissions` accepts a plain-text message and writes its RFC 5322
representation, recipients, queue row, and idempotency record in one
PostgreSQL transaction. It requires `delivery:write` and an
`Idempotency-Key`; replaying a key with the same normalized request returns
the original queue item, while reusing it for different content returns
`409`. Durable accounts must use a sender address on a verified tenant domain;
otherwise the API returns `403`. `GET /v1/submissions` lists recent submissions
for the authenticated tenant and requires `delivery:read`.

The endpoint acknowledges queue acceptance only. The local worker uses a
deterministic test processor, so `simulated_succeeded` is not remote SMTP
delivery. TLS-secured SMTP AUTH is implemented in the separate SMTP ingress
service; outbound worker integration, Sent-copy creation, and durable
delivery/DSN events remain incomplete.

The dashboard proxies `/api/backend/*` to the server-side `API_URL` origin
(default `http://127.0.0.1:8080` in development) to keep browser requests
same-origin. The browser never calls the API origin directly, so the dashboard
does not need permissive API CORS headers; do not add wildcard CORS as a
workaround. Set `API_URL` on the Next.js server to an absolute HTTP(S) origin
in non-development environments (use HTTPS in production). It is server-only
and is not exposed as `NEXT_PUBLIC_API_URL`. A development build sends the
local development principal when no bearer token is provided; the API accepts
it only when `CONTROL_PLANE_DEV_MODE=true`, which must not be enabled in
deployment. Other environments require a user-bound API key or OIDC access
token; the dashboard keeps pasted tokens in memory only.

## Adding an endpoint

1. Register the route in `internal/controlplane/routes.go`.
2. Authenticate and require the narrowest permission before loading resources.
3. Derive tenant scope from the authenticated principal.
4. Bound and strictly decode request bodies.
5. Return the documented status and error shape.
6. Add handler tests for success, malformed input, authentication,
   authorization, tenant isolation, and request IDs.
7. Update `openapi.yaml` and the phase-status record if behavior remains
   intentionally incomplete.

## Communication policy

No internal HTTP client or RPC transport currently has multiple consumers, so
Phase 08 does not add a generic client abstraction. Phase 04 event envelopes
remain the event contract; a durable publisher/outbox is not yet available.
Retries, idempotency, and distributed rate limiting must not be represented as
implemented guarantees while the current control-plane store is in memory.
