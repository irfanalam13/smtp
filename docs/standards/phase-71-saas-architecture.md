# Phase 71 — SaaS Architecture

Status: **Architecture research and proposal documented; implementation pending**

## Repository baseline

The repository already has a Go control plane, PostgreSQL tenant and
membership tables, scoped API keys, a Next.js dashboard, and a documented
control-plane/data-plane boundary. The control plane now uses PostgreSQL for
mailbox list and message metadata reads, but other control-plane state remains
in-memory. There are no organization/workspace, subscription, plan, feature
flag, or general-purpose permission-policy models yet. The architecture
overview explicitly keeps billing and analytics off the message-acceptance
path and favors a small number of independently scalable deployables over
premature microservices.

## Recommended SaaS model

- Treat the existing `tenants` record as the customer organization and the
  security, billing, quota, and lifecycle boundary. Do not create a duplicate
  organization identity until a distinct parent/child business requirement is
  approved.
- Introduce workspaces only as an optional tenant-owned scope when customers
  need to segment resources or delegate administrators. A workspace never
  replaces tenant isolation; resources remain tenant-bound.
- Keep the control plane authoritative for membership, roles, plans, flags,
  domains, credentials, and audit. The data plane consumes bounded,
  versioned policy/configuration and does not synchronously call billing,
  analytics, or dashboard services per message.
- Keep PostgreSQL as the default control-plane database and REST/OpenAPI as
  the primary external API. Continue the existing modular deployable
  architecture; add services only for independent scaling, trust, release, or
  failure boundaries supported by evidence.
- Use external identity and payment providers behind narrow interfaces. Do
  not build a password/token issuer or payment-card vault in this repository.

These are recommendations for the phase sequence, not a claim that the
multi-tenant SaaS platform is implemented. Phase 72 must validate the storage
isolation strategy before schema changes are treated as final.

## Phases and dependency order

71 defines ownership and service boundaries. Phase 72 implements tenant
isolation; Phase 73 adds organization/workspace membership; Phase 74 establishes
production identity; Phase 75 adds authorization policy; Phase 76 meters and
bills usage; Phase 77 defines ingress protection; Phase 78 completes REST;
Phase 79 may add GraphQL after stable resource APIs; Phase 80 publishes SDKs
from versioned contracts. Later capabilities such as templates, webhooks, and
customer analytics need their own requirements and acceptance criteria; they
must not be inferred from this phase list.

## Remaining work and why

Create and review a SaaS ownership/deployment decision, inventory every
tenant-owned table and data flow, define organization/workspace semantics,
assign owners for identity, authorization, billing, edge, API, and developer
experience, and set measurable launch gates for isolation, availability,
recovery, and cost.

This remains open because the repository contains useful foundations but not
the product hierarchy, operational deployment target, provider choices, or
cross-component evidence needed to make a safe implementation plan. A feature
list or architecture diagram is not evidence of SaaS isolation or production
readiness.

## Acceptance criteria

- Every authoritative resource has a named owner and tenant boundary.
- Tenant identity is derived from verified identity and cannot be selected by
  request payloads.
- Control-plane outages do not create an unbounded synchronous dependency in
  SMTP acceptance or delivery.
- Provider seams and service boundaries have an owner, failure behavior, and
  test strategy.
- Launch readiness is backed by tenant-isolation, load, failure, migration,
  restore, security, and operational evidence.

## Research references

- [Repository architecture overview](../architecture/phase-01/architecture-overview.md)
- [Repository service boundaries](../architecture/phase-01/service-boundaries.md)
- [Repository data and deployment model](../architecture/phase-01/data-and-deployment.md)
- [Control/data-plane decision](../architecture/adr/phase-01/0011-control-data-plane.md)
- [Tenant isolation proposal](../architecture/adr/phase-01/0014-tenant-isolation.md)
- [Phase implementation status](../phase-status.md)
