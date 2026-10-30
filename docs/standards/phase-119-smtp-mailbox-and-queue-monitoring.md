# Phase 119 — SMTP, Mailbox, and Queue Monitoring

Status: **API request metrics only; protocol and business-flow monitoring pending**

## Goal and scope

Monitor the email product's critical paths, not just host and HTTP health:
SMTP acceptance, mailbox access, queue processing, delivery attempts, storage,
and tenant fairness. Metrics must distinguish service health from individual
message content and avoid exposing customer data.

## Repository baseline

The API has process-local request counters, a latency histogram, and an active
request gauge. The repository has SMTP session and queue primitives, but no
centralized fleet metrics, mailbox service SLOs, queue dashboards, or
production mail-flow synthetic monitoring.

## Recommended design

Measure SMTP command outcomes and acceptance latency, TLS/auth failures,
message size class, queue depth/oldest age, lease/retry/terminal rates,
delivery response classes, mailbox operation latency, storage utilization,
and database pool saturation. Use bounded outcome/status categories; do not
label by address, message ID, raw MX hostname, tenant ID, or arbitrary SMTP
reply text. For per-tenant fairness, use access-controlled operational
analytics or sampled aggregates instead of unbounded time-series labels.

Alert on durable acceptance failures, queue age beyond the approved SLO,
growing terminal failures, mailbox errors, and storage exhaustion. Controlled
synthetic transactions must use owned domains and dedicated test mailboxes,
with no customer content or unsolicited external recipients.

## Remaining work and why

Instrument Rust protocol/worker/storage paths, centralize metrics, set SLOs,
and build safe synthetic checks and dashboards. This remains open because
business-flow telemetry and operational destinations are not connected.

## Acceptance criteria

- Critical SMTP, queue, mailbox, and storage SLIs are measured across replicas.
- Queue age and failure metrics are distinguishable from ordinary backlog
  fluctuations and have tested alert thresholds.
- Metrics cannot reveal recipients, message content, credentials, or
  high-cardinality customer identifiers.
- Synthetic mail flow is isolated, authorized, bounded, and auditable.
- A controlled dependency outage produces actionable signals without
  generating retry storms.

## Research references

- [Phase 16 Mail Queue Engine](phase-16-mail-queue-engine.md)
- [Durable mail queue architecture](../architecture/durable-mail-queue.md)
- [OpenTelemetry HTTP metrics conventions](https://opentelemetry.io/docs/specs/semconv/http/http-metrics/)
