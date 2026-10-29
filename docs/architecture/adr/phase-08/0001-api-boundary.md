# ADR 0001: API boundary and communication foundation

- Status: Accepted
- Date: 2026-10-28

## Context

The repository has a Go `net/http` control plane with versioned routes,
Phase 07 authentication/authorization, and an OpenAPI document. It has no
approved internal RPC transport or durable event publisher.

## Decision

Extend the existing `net/http` handlers with a bounded request-ID middleware,
stable nested error responses, strict JSON decoding, and explicit pagination
validation. Preserve `/v1` versioning and the existing Phase 07 security
boundary. Do not introduce gRPC, a gateway, service mesh, generic internal
client, durable idempotency, or a parallel event system without a real
consumer and a corresponding architecture decision.

## Consequences

New endpoints have predictable request and error behavior and can be tested
without a framework migration. Request IDs are suitable for correlation but
are not a complete Phase 09 tracing implementation. Durable idempotency,
distributed rate limiting, service-to-service authentication, and outbox
publication remain deferred until their storage and deployment consumers exist.
