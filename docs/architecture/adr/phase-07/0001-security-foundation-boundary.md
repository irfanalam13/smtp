# ADR 0001: Security foundation boundary

- Status: Accepted
- Date: 2026-03-08

## Context

The current platform has a Go control plane with development identity,
scoped API keys, tenant-scoped resources, and an in-memory implementation.
Production OIDC, service workload identity, and durable secret infrastructure
are not configured.

## Decision

Keep current authentication and authorization ownership in the control plane,
and provide a small reusable security package for typed identities, service
identity classification, resource tenant matching, and default-deny
authorization. Do not introduce passwords, JWTs, an OAuth server, or a new
identity provider before the unresolved Phase 01 identity decisions are made.

API keys remain high-entropy bearer credentials with one-time display,
constant-time verifier comparison, scope, expiration, and revocation. The
development identity adapter is fail-closed in production.

## Consequences

- Existing control-plane behavior remains compatible while security decisions
  become explicit and testable.
- Production deployments still need an approved OIDC provider, durable API-key
  persistence, workload identity, and secret/KMS integration.
- In-memory audit events are suitable only for local behavior and tests until a
  durable audit adapter is wired to PostgreSQL and an access-controlled sink.
