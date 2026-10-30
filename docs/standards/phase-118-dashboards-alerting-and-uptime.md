# Phase 118 — Dashboards, Alerting, and Uptime Monitoring

Status: **No deployed dashboards, alert routing, or external uptime checks**

## Goal and scope

Turn service telemetry into actionable, owned operational signals. Dashboards
should support service health and incident diagnosis; alerting should page only
on actionable symptoms with runbooks and escalation ownership.

## Repository baseline

The API exposes local health endpoints and Prometheus text metrics. The
repository has no configured central dashboard, alert manager, public uptime
probe, on-call rotation, or verified notification path.

## Recommended design

Define user-visible SLIs and SLOs before alerts: successful API operations,
SMTP acceptance at the durable boundary, delivery delay, mailbox read/write
availability, and queue age. Separate symptom alerts from capacity forecasts
and low-priority diagnostics. Use error-budget burn rates, dependency status,
and saturation panels while avoiding alerts on every transient failure.

Use external probes from more than one network vantage point. Liveness,
readiness, and customer-facing synthetic checks must have distinct meanings.
Synthetic email tests should use controlled recipients/domains and never
generate unsolicited traffic. Every page needs a named responder, severity,
runbook, and tested notification route.

## Remaining work and why

Select and deploy dashboard/alert systems, define on-call owners, connect
telemetry, and rehearse incident workflows. This remains open because no
production environment, alert destination, SLO, or operating team is
configured.

## Acceptance criteria

- Dashboard shows service-level traffic, errors, latency, saturation, and
  dependencies across all active instances.
- Every paging alert maps to a customer-impacting SLI or urgent security event,
  has an owner and tested runbook.
- External uptime checks detect DNS, TLS, routing, and application failures.
- Alert delivery and escalation are periodically exercised.
- Synthetic checks have safe destinations, rate limits, and cleanup policy.

## Research references

- [Google SRE Workbook](https://sre.google/workbook/table-of-contents/)
- [Kubernetes probes](https://kubernetes.io/docs/concepts/configuration/liveness-readiness-startup-probes/)
- [Phase 110 Failover, Rolling Updates, and Self-Healing](phase-110-failover-rollouts-self-healing.md)
