# Phase 77 — API Gateway

Status: **Versioned Go API and middleware foundations exist; managed edge and distributed controls pending**

## Repository baseline

The Go `net/http` API exposes versioned `/v1` REST routes, request IDs,
authentication and authorization middleware, strict JSON decoding, bounded
pagination, and a documented error contract. No centralized gateway,
distributed rate limiter, WAF deployment, or production edge configuration
is present. The architecture leaves TLS ingress to a managed edge or reverse
proxy and avoids a second HTTP framework or service mesh without need.

## Recommended gateway boundary

Use a managed load balancer/reverse proxy/WAF for internet-facing TLS,
connection limits, IP/network controls, and coarse denial-of-service
mitigation. Keep application identity, tenant authorization, schema
validation, business idempotency, and resource-level limits in the Go API.
Do not build a bespoke gateway microservice solely to proxy the current API.

Apply request/body/time limits, per-credential and per-tenant quotas, bounded
concurrency, safe header forwarding, canonical request IDs, and redacted
structured logs. Any distributed limiter must have explicit failure behavior:
security-sensitive write limits fail closed or use a documented conservative
local bound; mail acceptance does not depend on a nonessential analytics
service. Caching is opt-in only for safe, tenant-scoped reads and cannot cache
authorization decisions across identity changes.

## Remaining work and why

Select the deployment edge and trust-proxy model; define trusted forwarded
headers, TLS/certificate ownership, WAF/rules, rate-limit dimensions and
failure behavior, request/response size/time budgets, cache policy, and
operational ownership. Add deployment configuration and tests at the actual
edge, including spoofed forwarding headers and tenant-key abuse.

This remains open because these controls depend on the target cloud/runtime
and cannot be verified from API middleware alone. A proxy deployed with the
wrong trust boundaries can make client-supplied identity headers appear
authoritative or expose backend services directly.

## Acceptance criteria

- Only approved ingress can reach application instances, and forwarded
  identity/client-IP headers are trusted only from configured proxies.
- TLS, request limits, timeouts, rate limits, and error responses are tested
  at the deployed edge.
- Limits distinguish tenant, credential, and endpoint costs without allowing
  tenant ID spoofing or shared-IP starvation.
- Gateway failures and policy updates have monitored, documented behavior.
- Logs and traces exclude credentials, message contents, and sensitive
  request bodies.

## Research references

- [Repository API and communication standards](api-and-integration-standards.md)
- [Repository deployment topology](../architecture/phase-01/data-and-deployment.md)
- [OWASP API Security Top 10](https://owasp.org/API-Security/)
