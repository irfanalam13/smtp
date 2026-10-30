# Phase 115 — Rate Limiting, Autoscaling, and Performance Benchmarks

Status: **Local safeguards exist; distributed enforcement and load evidence pending**

## Goal and scope

Protect tenants and downstream services under load with explicit rate/concurrency
budgets, carefully tuned autoscaling, and reproducible performance tests.
Limits must reflect the identity and resource being protected, not merely
requests handled by one process.

## Repository baseline

The Go API has process-local rate-limit state for selected control-plane
operations. It is not coordinated across replicas and does not provide global
tenant quotas. No production autoscaler, public capacity targets, or
representative load benchmark suite is present.

## Recommended design

Define separate request, concurrent-operation, recipient, message-size,
outbound-domain, and storage budgets where the threat or dependency differs.
Specify burst behavior, retry headers, fairness, exemptions, and operator
overrides. Distributed enforcement must be atomic or conservatively bounded;
failure behavior must not silently disable protection or block all mail.

Autoscale stateless services on validated demand signals with minimum capacity,
stabilization windows, scale-down protection for in-flight work, readiness and
draining, and hard caps derived from downstream budgets. A queue backlog or CPU
signal alone is insufficient if database connections or remote MX limits are
already saturated. Benchmarks must include tail latency, errors, resource use,
and data correctness, not just requests per second.

## Remaining work and why

Approve quotas, SLOs, abuse policy, provider/runtime configuration, and test
ownership. Implement shared enforcement only with a selected coordination
backend and failure model. This remains open because local process state cannot
enforce fleet-wide policy, and no production metrics/load target is available.

## Acceptance criteria

- Limit decisions are tenant/resource scoped, auditable, and tested across
  multiple API/worker replicas.
- Store/coordination failure has a reviewed, bounded fail-open or fail-closed
  behavior for each operation class.
- Autoscaling respects DB, queue, DNS, object-store, and destination budgets.
- Load tests document p50/p95/p99, error rate, saturation, and test revision.
- Scale-in drains in-flight SMTP sessions and queue leases safely.

## Research references

- [Kubernetes Horizontal Pod Autoscaling](https://kubernetes.io/docs/concepts/workloads/autoscaling/horizontal-pod-autoscale/)
- [Phase 110 Failover, Rolling Updates, and Self-Healing](phase-110-failover-rollouts-self-healing.md)
- [Phase 98 Enterprise Scalability](phase-98-enterprise-scalability.md)
