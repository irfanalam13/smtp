# Phase 07 threat model

This model uses STRIDE against the currently implemented control-plane
boundary.

| Threat | Asset | Mitigation | Residual risk |
|---|---|---|---|
| Spoofing | API keys and development identity | CSPRNG keys, constant-time verification, expiry, revocation, dev-mode production guard | OIDC and operational key rotation remain deferred |
| Tampering | Tenant resources and authorization decisions | Typed identity, default-deny authorization, tenant resource matching, store checks | In-memory state is not a production persistence boundary |
| Repudiation | Administrative and key operations | Structured audit events with actor, tenant, action, and target | Current adapter retains events in process memory |
| Information disclosure | Secrets and message metadata | One-time secret display, redacted key listings, bounded message summaries, generic auth errors | Deployment logging and database controls require production integration |
| Denial of service | HTTP and DNS operations | Per-tenant/IP rate limits, bounded request bodies, resolver timeouts | Distributed abuse requires edge controls |
| Privilege escalation | Roles, scopes, and tenant data | Explicit scopes, service identity classification, cross-tenant denial | Full policy administration is deferred |

## Assumptions requiring validation

- Production identity is supplied by an approved OIDC provider.
- API-key storage is moved to the authoritative database with uniqueness and
  transaction guarantees.
- Audit events are persisted durably and exported to an access-controlled sink.
- TLS termination and certificate lifecycle are supplied by the deployment
  boundary or an approved service.
