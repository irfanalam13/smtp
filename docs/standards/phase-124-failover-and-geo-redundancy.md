# Phase 124 — Failover Automation and Geo-Redundancy

Status: **No production failover automation or geo-redundant service**

## Goal and scope

Provide controlled recovery from availability-zone or regional failures without
split-brain, unsafe duplicate delivery, or silent data loss. Automated
failover must be a consequence of tested ownership and fencing, not simply a
health-check threshold.

## Repository baseline

Health/readiness and process shutdown are local foundations. The repository
does not deploy a cluster, multi-region database, global traffic manager,
failover controller, or tested disaster-recovery infrastructure.

## Recommended design

Define failure detectors, quorum or provider promotion rules, single-writer
fencing, traffic cutover, queue ownership, idempotency, operator override,
rollback, and failback. Separate service restoration from data promotion.
Require explicit safety gates when replication lag exceeds the accepted data
loss budget. Ensure DNS, certificates, identity, key access, and support
communications also work in the recovery region.

## Remaining work and why

Build the selected provider topology in an isolated environment and prove
failover/failback under controlled failure. This remains open because no
cluster/account/network or approved RPO/RTO exists, and automation without
fencing could cause data divergence or duplicate external actions.

## Acceptance criteria

- Failover never permits two writable primaries for a single-writer data set.
- Replication lag is evaluated against explicit data-loss policy before
  promotion.
- SMTP ingress, queue workers, APIs, identity, secrets, and observability
  transition together or fail closed with documented service behavior.
- Failback reconciles state and is exercised without destructive operations.
- Every automation has audit evidence, bounded blast radius, and operator
  abort/override.

## Research references

- [PostgreSQL high availability](https://www.postgresql.org/docs/current/high-availability.html)
- [Phase 110 Failover and Self-Healing](phase-110-failover-rollouts-self-healing.md)
- [Phase 123 Point-in-Time Recovery](phase-123-point-in-time-recovery.md)
