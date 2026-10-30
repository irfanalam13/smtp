# Phase 164 — Geo Routing

Status: **No regional endpoints or geographic routing policy deployed**

## Recommended design

Route only stateless or explicitly region-owned traffic. Prefer provider
latency measurements over hard-coded geographic assumptions and support
health-aware failover. Keep tenant home-region and data-residency policy
authoritative: nearest region must not override where a user's data is allowed
to be processed. Account for resolver location, VPNs, mobile networks, and
DNS caching.

## Remaining work and why

Define allowed regions by tenant/data class, endpoint health, user-routing
signals, privacy policy, and fallback behavior. This remains open because
region mapping, customer residency commitments, and production telemetry do
not exist.

## Acceptance criteria

- Geo routing never bypasses tenant residency or authorization policy.
- Routing is based on tested health/latency data with deterministic fallback.
- Resolver geography inaccuracies and cached answers are documented.
- SMTP and API traffic policies are independently defined.
- Regional route decisions are observable without exposing personal data.

## Research references

- [Route 53 latency-based routing](https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/routing-policy-latency.html)
- [Phase 178 Regional Compliance](phase-178-regional-compliance.md)
