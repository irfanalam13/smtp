# ADR-0018: Core configuration and event contracts

- Status: Proposed
- Date: 2026-10-28

## Context

Services need deterministic configuration validation, safe local secret access,
and compatible asynchronous event contracts without coupling foundational code
to a broker that has not been selected.

## Decision

Use built-in typed defaults overridden by environment variables for the shared
baseline configuration. Validate before startup and reject invalid production
loopback binding. Represent events with UUID identity, explicit type and
schema version, UTC occurrence time, producer identity, optional correlation
context, and typed payload. Expose a publisher contract whose receipt is not
claimed durable until a transport adapter confirms it.

## Consequences

Configuration is simple and testable, but file and command-line sources remain
service-specific until required. Event consumers must implement idempotency and
schema compatibility. No event is silently dropped by the shared contract, but
durable publication requires a later adapter/outbox implementation.
