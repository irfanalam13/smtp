# Phase 78 — REST API Platform

Status: **Versioned REST/OpenAPI foundation exists; broad durable resource APIs pending**

## Repository baseline

The Go API uses `/v1`, strict JSON input, request IDs, bounded list
pagination, a consistent error envelope, authentication/authorization
middleware, and an OpenAPI document. PostgreSQL-backed mailbox reads and
idempotent plain-text message submission into the durable queue are
implemented. Dashboard API requests use a same-origin Next.js rewrite. Other
control-plane state is still in-memory; durable organization, identity,
billing, policy, analytics, template, webhook, and administrative resource
APIs are not complete.

## Recommended API contract

Keep REST/JSON with OpenAPI as the canonical public contract. Add resources
only when their owning model, authorization policy, persistence, lifecycle,
and operational semantics are defined. Use stable IDs, explicit tenant scope
from the principal, bounded cursor pagination for large collections,
idempotency keys for retryable writes, documented error codes, consistent
request IDs, deprecation/version policy, and additive compatibility.

Treat OpenAPI and handler behavior as a tested pair: validate the document,
exercise schemas and authorization, and generate client contract fixtures.
Avoid returning internal database models. A public endpoint is not complete
until its persistence, policy, rate/size bounds, audit, error contract, and
operational ownership are implemented.

## Remaining work and why

Create an API inventory by resource and owning phase; prioritize durable
tenant/identity/workspace APIs before dependent resources; complete
authentication, authorization, audit, pagination, idempotency, and versioning
contracts; and add contract/integration tests. Analytics, templates, and
webhooks need separately defined phases and must not be added as speculative
routes simply because they appear in a high-level platform description.

The submission endpoint is an initial bounded operation, not a complete
delivery API: queue acceptance is distinct from delivery, and the local worker
simulates results. This remains open because the current OpenAPI surface is a
partial API for development/control-plane, submission, and mailbox read
workflows, not a complete SaaS contract. Publishing incomplete resources
before durable ownership and access rules are ready creates breaking APIs and
security gaps.

## Acceptance criteria

- Every operation has a stable schema, authentication requirement, permission,
  tenant scope, bounded input/output, documented errors, and owner.
- OpenAPI validation and representative consumer contract tests run in CI.
- Breaking changes are versioned/deprecated with a migration window.
- Pagination is stable under concurrent updates; retryable mutations are
  idempotent where required.
- No endpoint exposes another tenant's resources or internal secret data.

## Research references

- [OpenAPI Specification 3.1.1](https://spec.openapis.org/oas/v3.1.1.html)
- [Repository API and communication standards](api-and-integration-standards.md)
- [Current API guide](../api/README.md)
