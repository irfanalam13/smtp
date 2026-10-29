# ADR-0011: Control-plane and data-plane separation

- Status: Proposed
- Date: 2026-10-28

## Context

Every-message synchronous calls to tenant APIs, billing, analytics, or dashboards would make delivery depend on nonessential systems.

## Decision

The control plane owns configuration and authorization. The data plane consumes signed, versioned policy snapshots and durable control events. It may continue accepted delivery during a bounded control outage; it fails closed for new unsafe acceptance when the policy lease expires. Emergency suspension has a priority propagation path.

## Consequences

Availability improves, but revocations are eventually propagated and require explicit lease duration, kill switches, monitoring, and reconciliation. Policy snapshots must not become an unbounded authorization cache.
