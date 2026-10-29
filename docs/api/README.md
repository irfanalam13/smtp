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

The machine-readable contract is [openapi.yaml](openapi.yaml). It documents
the current development control-plane surface and does not claim production
OIDC or durable Go-to-queue submission.

Mailbox lifecycle, alias, shared-access, folder, and quota routes are specified
in the [Phase 41 architecture contract](../standards/phase-41-mailbox-architecture.md)
but are not currently exposed as durable API operations. Existing mailbox read
routes use in-memory state; do not build clients that assume durable mailbox
provisioning or shared-mailbox enforcement is available.

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
