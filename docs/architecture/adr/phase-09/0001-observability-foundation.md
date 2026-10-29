# ADR 0001: Dependency-light observability foundation

- Status: Accepted
- Date: 2026-10-29

## Context

The repository has Rust structured logging, distinct operation identifiers, a
Go HTTP API, and no deployed metrics, tracing, log aggregation, or dashboard
backend. The local Compose environment currently contains only PostgreSQL.

## Decision

Add bounded, vendor-neutral observability at actual API and health boundaries
without introducing a hosted backend or a new telemetry dependency. The Go API
exports aggregate Prometheus text metrics and structured completion logs.
Liveness and readiness remain separate. Rust continues using the existing
`tracing` initialization and context types.

## Alternatives considered

- OpenTelemetry SDK and collector: deferred until a collector and second
  service boundary exist.
- Prometheus client dependency: deferred because the current metric surface is
  small and a dependency-free exposition keeps the local foundation minimal.
- Grafana/Prometheus Compose services: deferred because they would add
  unowned local infrastructure and do not provide production retention or
  access control.

## Consequences

The API is observable in local and single-process deployments, with
cardinality-safe metrics and testable health behavior. Counters are process
local, traces are not exported, and dashboard/alert files require a selected
Prometheus-compatible deployment. Phase 10 must validate overhead, retention,
access control, and distributed aggregation before production claims.
