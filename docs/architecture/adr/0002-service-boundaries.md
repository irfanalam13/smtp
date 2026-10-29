# ADR 0002: Service boundaries

- Status: Accepted
- Date: 2026-10-28

## Context

SMTP protocol handling has different runtime and performance needs from administrative APIs and browser UI.

## Decision

Use Rust for protocol-facing services, Go for the control plane, and Next.js for the dashboard.

## Alternatives considered

A single language would reduce toolchain count but would not optimize each boundary equally.

## Consequences

Contracts must be explicit, CI covers three ecosystems, and local development needs consistent version pinning.
