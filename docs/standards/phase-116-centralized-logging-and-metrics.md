# Phase 116 — Centralized Logging and Metrics

Status: **API process-local metrics enhanced; centralized telemetry pending**

## Goal and scope

Provide reliable, privacy-aware metrics and logs across replicas and services,
with defined ownership, retention, access control, and alerting. This phase
begins the observability work in 116–120.

## Repository baseline and code slice

The Go API emits structured request-completion logs and exposes process-local
Prometheus text at `/metrics`. Its metrics now include a request-duration
histogram in seconds, active-request gauge, request/failure counters, and
bounded response status classes. Labels do not include request IDs, tenant
IDs, message IDs, email addresses, or raw paths. The histogram buckets and
active-request gauge have Go tests.

The Rust services have local structured tracing. There is no shared collector,
central log/metric backend, durable retention policy, or fleet aggregation.
The metrics endpoint remains private-network-only; it is not authenticated as
a public API.

## Recommended design

Collect service metrics, logs, and resource/saturation signals through a
vendor-neutral pipeline with bounded queues and explicit drop/backpressure
behavior. Define schemas and cardinality budgets. Redact credentials, tokens,
message content, and personal data before export; document access and retention.
Use low-cardinality route templates and bounded labels. Keep diagnostic
telemetry separate from customer-facing audit records.

The current classic histogram is intentionally small and dependency-free.
Fleet-wide percentile calculation requires scraping each instance and
aggregating bucket counters; local values alone are not a fleet health record.
Choose an OpenTelemetry collector/export path only after backend and
deployment/network choices are approved.

## Remaining work and why

Deploy and operate collectors, central storage, dashboards, retention,
redaction, access policy, and alert routes. Validate behavior under exporter
outage and cardinality pressure. This remains incomplete because no production
telemetry backend, account, retention policy, or on-call ownership is
configured.

## Acceptance criteria

- Metrics have stable names/units and bounded labels; no customer content or
  direct identifiers are exported.
- Logs can be correlated with bounded request/trace identifiers without
  logging credentials or message bodies.
- Central collection survives instance replacement and handles backend outage
  with bounded local memory and explicit drop telemetry.
- `/metrics` is reachable only from authorized operational networks.
- Dashboards and alerts use aggregated multi-instance metrics and have tested
  owner/runbook routes.

## Research references

- [Prometheus metric naming practices](https://prometheus.io/docs/practices/naming/)
- [Prometheus histograms](https://prometheus.io/docs/practices/histograms/)
- [OpenTelemetry metrics data model](https://opentelemetry.io/docs/specs/otel/metrics/data-model/)
- [Repository observability status](../observability/README.md)
