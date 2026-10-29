# ADR-0010: Small independently scalable service architecture

- Status: Proposed
- Date: 2026-10-28

## Context

SMTP protocol handling, remote delivery, and administration have materially different load, trust, and failure characteristics. A fine-grained microservice fleet would add distributed failure modes before workload evidence exists.

## Decision

Use a small number of independently scalable deployables with modular internals: control plane, SMTP/data ingress, queue/delivery, and dashboard. Combine modules during local development where it does not weaken contracts.

## Alternatives

Modular monolith reduces operations but couples failure domains. Fine-grained microservices improve theoretical scaling but require many contracts, operators, and distributed transactions.

## Consequences

The team owns explicit interfaces and separate resource pools without premature service proliferation. A module may split only for scaling, security, release, or incident-isolation evidence. This is reversible if ownership boundaries remain explicit.
