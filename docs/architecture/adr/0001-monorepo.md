# ADR 0001: Monorepo

- Status: Accepted
- Date: 2026-10-28

## Context

The platform contains protocol services, a control-plane API, shared types, and a dashboard that must evolve together.

## Decision

Keep these components in one repository with explicit workspace boundaries.

## Alternatives considered

Separate repositories would isolate release cycles but make shared contracts and coordinated changes harder to review.

## Consequences

One change can update all dependent components atomically. CI must remain scoped and reproducible for each workspace.
