# ADR-0019: Shared logging and bounded shutdown

- Status: Proposed
- Date: 2026-10-28

## Context

Multiple executables need consistent structured logs and cancellation behavior,
while Phase 09 owns exporters, metrics, dashboards, and alerting.

## Decision

Use `tracing` and `tracing-subscriber` for application-owned subscriber
initialization, with compact development output and JSON output for staging or
production. Libraries emit events but do not initialize global subscribers.
Use a small Tokio watch-based shutdown signal with an explicit bounded wait
helper.

## Consequences

Logs are compatible with later telemetry integration without implementing a
backend now. Global subscriber initialization remains process-wide and must be
tested in isolated executables. Shutdown ownership is explicit, but each
service remains responsible for stopping its consumers before dependencies.
