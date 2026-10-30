# Phase 174 — Automatic Failover

Status: **No automated regional failover controller or tested routing cutover**

## Recommended design

Automate only after detecting failures with independent signals and proving
single-writer fencing, replication freshness, quorum/promotion, traffic
cutover, and rollback. Use operator-controlled safety gates when data loss
exceeds policy. DNS/global traffic failover is not database promotion and
does not ensure queues, identity, keys, or certificates are ready.

## Remaining work and why

Implement and test per-service failover under isolated failure scenarios,
including false-positive health checks and partial-region outage. This remains
open because no infrastructure or approved blast-radius/recovery policy
exists.

## Acceptance criteria

- Failover is idempotent, audited, bounded, and protected against split-brain.
- Partial outages do not trigger unsafe global promotion.
- SMTP/queue ownership and duplicate prevention are proven.
- Failback has a separate explicit procedure and reconciliation.
- Operator abort and manual recovery are tested.

## Research references

- [Route 53 health checks and DNS failover](https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/dns-failover.html)
- [Phase 124 Failover and Geo-Redundancy](phase-124-failover-and-geo-redundancy.md)
- [Phase 110 Failover and Self-Healing](phase-110-failover-rollouts-self-healing.md)
