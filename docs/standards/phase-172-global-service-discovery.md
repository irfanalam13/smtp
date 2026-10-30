# Phase 172 — Global Service Discovery

Status: **No regional service registry or global discovery configuration exists**

## Recommended design

Keep intra-region discovery within each cluster and expose only deliberate
regional endpoints globally. Use health-aware routing with authenticated
service identity, certificate lifecycle, bounded DNS caching, and explicit
endpoint ownership. Discovery must never imply authorization. Avoid global
registries that expose private service addresses or cross residency boundaries.

## Remaining work and why

Define cluster/region naming, endpoint contracts, certificate and workload
identity, health semantics, and failover ownership. This remains open because
there are no clusters, service mesh, or deployment identity configured.

## Acceptance criteria

- Only intended public endpoints are discoverable from global routing.
- Internal services authenticate peer identity independent of network
  location.
- Stale endpoint and DNS failure behavior is bounded and tested.
- Health reflects ability to serve safely, not just process liveness.
- Endpoint lifecycle changes are audited and recoverable.

## Research references

- [Amazon EKS Best Practices](https://docs.aws.amazon.com/eks/latest/best-practices/introduction.html)
- [Phase 106 Cluster Deployment](phase-106-cluster-deployment-service-discovery.md)
- [Phase 101 Zero Trust Service Identity](phase-101-zero-trust-service-identity.md)
