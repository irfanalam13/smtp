# Observability operations

## Initial signals

The API overview should use request rate, 5xx count, response status class,
request duration, and readiness. These signals are intentionally aggregate and
avoid sensitive or unbounded labels.

The provisional alert rules in
[`../../observability/prometheus/alerts.yml`](../../observability/prometheus/alerts.yml)
are examples for a Prometheus-compatible deployment. They require a scraper
and are not active in the local Compose environment.

## Alert severity

- **Critical:** sustained service unavailability or readiness failure that
  prevents the service from accepting required work.
- **Warning:** elevated server errors or sustained latency requiring
  investigation.
- **Informational:** telemetry or capacity condition without immediate user
  impact.

Thresholds are provisional until Phase 10 load and failure validation produces
real workload evidence.

## Runbook: API unavailable

1. Check `/health/live` and `/health/ready`.
2. Inspect API process logs using the request ID from a failing response.
3. Check PostgreSQL availability if the endpoint or deployment requires it.
4. Confirm the process is not running with development identity mode in a
   production environment.
5. Restore only after readiness returns `200` and a sanitized authenticated
   request succeeds.

## Known gaps

There is no centralized log retention, trace backend, alert notification
route, distributed metrics aggregation, or dashboard server in the local
environment. These require a selected deployment target and access-controlled
operational infrastructure.
