# Phase 120 — Capacity Planning and Operational Insights

Status: **No historical fleet telemetry or forecasting pipeline**

## Goal and scope

Use reliable historical metrics and incident evidence to forecast capacity,
detect operational anomalies, and guide human action. Automated insights must
be explainable, privacy-preserving, and subordinate to explicit service
limits and operator controls.

## Repository baseline

The API metrics are process-local and reset at process restart. No central
historical metrics store, capacity forecasting, anomaly detection, or
operational analytics pipeline exists. Consequently, the current measurements
cannot establish growth trends or service-wide capacity.

## Recommended design

Build forecasts only after stable SLI and saturation data are collected with
consistent service/region dimensions and known retention. Forecast queue age,
database connections, storage, bandwidth, and workload growth with confidence
bounds and documented assumptions. Detect anomalies against seasonality and
deployment changes; prioritize precision and operator explanation over
automatic remediation.

Separate operational telemetry from tenant business analytics. Apply retention,
access controls, aggregation, data minimization, and deletion rules. A model or
alert must not inspect message bodies or make customer-impacting decisions
without a separately approved product and safety design.

## Remaining work and why

First establish centralized metrics, labels, history, SLOs, and reliable
capacity measurements. Then validate forecasting/anomaly methods against
historical incidents and planned load. This remains open because local counters
are ephemeral and there is no validated historical dataset or operations
ownership.

## Acceptance criteria

- Forecasts report assumptions, confidence intervals, data freshness, and
  known blind spots.
- Alerts are evaluated against historical incidents and produce low-noise,
  actionable notifications.
- Recommendations do not automatically change routing, quotas, or mail delivery
  without an independently reviewed control.
- Telemetry retention and access follow approved privacy and security policy.
- Capacity plans include headroom, scale lead time, dependency limits, and
  recovery scenarios.

## Research references

- [Google SRE Workbook](https://sre.google/workbook/table-of-contents/)
- [OpenTelemetry metrics data model](https://opentelemetry.io/docs/specs/otel/metrics/data-model/)
- [Phase 116 Centralized Logging and Metrics](phase-116-centralized-logging-and-metrics.md)
