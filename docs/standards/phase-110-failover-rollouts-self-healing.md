# Phase 110 — Failover, Rolling Updates, and Self-Healing

Status: **Local health endpoints and shutdown handling exist; automated cluster recovery and production drills pending**

## Repository baseline

The Go API exposes liveness/readiness checks and graceful shutdown. These are
single-process foundations. There is no production orchestrator, multi-node
failover, rolling deployment policy, circuit-breaker fleet, chaos program, or
verified recovery procedure. Health endpoints alone do not establish HA.

The API now has an atomic readiness gate that can withdraw readiness before
shutdown, waits briefly for load-balancer propagation, and then performs a
bounded graceful shutdown. This supports request draining for that process;
it does not orchestrate node replacement or prove load-balancer behavior.

## Recommended design

Define service-level indicators/objectives and recovery objectives before
automating remediation. Liveness should detect irrecoverable process failure;
readiness should withdraw instances that cannot safely accept work. Dependency
outages should not trigger restart storms. Drain connections and queue leases,
use bounded retries with jitter and circuit breakers, and make migrations
expand/contract compatible. Autoscaling must respect database and downstream
capacity. Self-healing actions must be observable, bounded, and reversible.
Run fault-injection only in isolated/staging environments or under an approved
production experiment with a defined blast radius and abort criteria.

## Remaining work and why

Select runtime/provider; define SLOs, alert ownership, runbooks, RTO/RPO,
deployment/rollback strategy, and fault hypotheses. Implement deployment
automation, dependency-aware health semantics, drains, migration gates,
canaries, alerting, and repeated failover/recovery exercises. This remains
open because infrastructure and operator/on-call ownership are not configured.

## Acceptance criteria

- Node, zone, database, broker, DNS, and object-store failures have bounded,
  tested recovery and explicit user-visible behavior.
- Rolling deploy and rollback preserve data, in-flight SMTP sessions, and
  leased queue work.
- Health checks avoid cascading restart and retry storms.
- Alert/runbook/on-call ownership is exercised; approved RTO/RPO and SLOs are
  met by measured evidence.
- Chaos experiments are authorized, isolated/bounded, and have abort/restore
  procedures.

## Research references

- [Kubernetes probes](https://kubernetes.io/docs/concepts/configuration/liveness-readiness-startup-probes/)
- [Google SRE Workbook](https://sre.google/workbook/table-of-contents/)
- [Phase 99 Production Hardening](phase-99-production-hardening.md)
