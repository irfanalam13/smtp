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
PostgreSQL-backed API keys and, when configured, Keycloak OIDC bearer access
tokens for pre-provisioned users. OIDC issuer, signature, expiration, and
dedicated API audience are verified; current user status, tenant membership,
and permissions come from PostgreSQL. `X-Tenant-ID` selects an active
membership when a user belongs to multiple tenants; it does not grant
membership. Mailbox list/message reads additionally check user-bound mailbox
permissions. Browser login/sessions, automated user provisioning, SAML, and
durable domain/control-plane state remain unimplemented.

To enable Keycloak token verification, set both `OIDC_ISSUER_URL` and
`OIDC_API_AUDIENCE`. Configure the Keycloak realm to issue an API access token
whose `aud` contains the dedicated API audience and whose `typ` claim is
`Bearer`; the browser client's ID token is rejected. Users must already have a row in `users` with
the exact issuer and subject and an active `tenant_memberships` row. Migration
`000015` adds the identity issuer column. Do not link identities by email.

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
  --scopes delivery:read
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

Mailbox lifecycle, alias, folder mutation, import/export, and synchronization
routes remain unimplemented. Usage counters and DNS diagnostic cache remain
process-local; this slice does not imply production readiness.

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
delivery. SMTP AUTH, outbound transport integration, Sent-copy creation, and
durable delivery/DSN events remain incomplete.

The dashboard proxies `/api/backend/*` to the server-side `API_URL` origin
(default `http://127.0.0.1:8080`) to keep browser requests same-origin. A
development build uses the local development principal when no bearer token
is provided. Other environments require a user-bound API key or OIDC access
token; the dashboard keeps pasted tokens in memory only.

## Adding an endpoint

1. Register the route in `internal/controlplane/handler.go`.
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
