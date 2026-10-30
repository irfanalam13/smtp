# Phase 79 — GraphQL API

Status: **Not implemented; additive GraphQL value and operational cost need validation**

## Repository baseline

The public API is REST/JSON with OpenAPI and has no GraphQL schema, resolver,
subscription transport, or federation. The platform is not yet a set of
independently owned GraphQL services. REST remains the existing supported
contract.

## Recommended implementation boundary

Treat GraphQL as an optional additive query interface, not a replacement for
REST or the source of truth. Before implementation, identify concrete client
workflows that materially benefit from graph-shaped reads. Reuse the same
principal, tenant, permission, and domain services as REST; each resolver
must enforce resource-level authorization. Keep mutations narrow and
idempotent, and keep SMTP delivery and other durable side effects out of
resolver orchestration.

Do not introduce schema federation until separate teams/services own stable
subgraphs and independent deployment is justified by architecture evidence.
Bound query depth, complexity, aliases, input sizes, execution time, and
concurrency; prevent N+1 access with request-scoped batching; define pagination
and error redaction; disable or restrict introspection according to deployment
policy. Subscriptions require a durable event source, authorization on
subscribe and delivery, revocation behavior, and backpressure—not merely a
WebSocket handler.

## Remaining work and why

Demonstrate product need, select a maintained Go implementation, define schema
ownership, query budgets, authorization mapping, pagination, caching, mutation
semantics, subscription transport, introspection policy, and operational
metrics. Build only after REST resources and tenant/RBAC contracts are stable;
measure query cost and compare with REST before claiming performance benefits.

This remains open because GraphQL adds a second public contract and a
resource-amplification surface while the underlying SaaS APIs and authorization
model are still incomplete. Federation and subscriptions have no current
service/event boundary to support them.

## Acceptance criteria

- GraphQL cannot return any data that the same principal cannot read through
  the owning REST/domain policy.
- Query complexity, depth, time, aliases, and batch sizes are bounded and
  tested against adversarial documents.
- Per-field authorization and tenant context are tested for nested object
  traversal and guessed IDs.
- N+1 behavior, caching invalidation, and execution metrics are measured.
- Subscriptions, if implemented, enforce per-event authorization,
  backpressure, reconnect, and revocation.

## Research references

- [GraphQL Specification](https://spec.graphql.org/)
- [Phase 75 RBAC and permission engine](phase-75-rbac-permission-engine.md)
- [Phase 78 REST API platform](phase-78-rest-api-platform.md)
