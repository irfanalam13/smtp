# Phase 01 — System Architecture

Status: **Proposed, implementation-aligned**  
Date: 2026-10-28  
Scope: architecture for the email delivery foundation, with extension paths for hosted mailboxes and a managed email cloud.

This directory is the Phase 01 architectural reference. It supplements, rather than replaces, the implementation-specific documents in [`../`](../). Existing accepted repository decisions remain authoritative where this specification says **accepted**. Decisions marked **provisional** require validation in later phases.

## Reading order

1. [Architecture overview](architecture-overview.md)
2. [Requirements and assumptions](requirements-and-assumptions.md)
3. [Service boundaries](service-boundaries.md)
4. [Message lifecycle and delivery semantics](message-lifecycle.md)
5. [Technology stack](technology-stack.md)
6. [Security architecture](security-architecture.md)
7. [Reliability and failure modes](reliability-and-failure-modes.md)
8. [Data ownership and deployment](data-and-deployment.md)
9. [Observability, licensing, risks, and phase dependencies](governance-and-evolution.md)
10. [Architecture decision records](adr/)

## Product scope

The initial production-capable product is a **tenant-scoped transactional email delivery service**:

- authenticated API and SMTP submission;
- verified sending domains and identities;
- durable acceptance and outbound queueing;
- controlled SMTP delivery with retries and delivery events;
- bounce/suppression and abuse-control foundations;
- usage, audit, and operational visibility.

Inbound SMTP reception is an opt-in extension and is not required for initial public launch. Hosted mailboxes, IMAP/POP3, webmail, and full inbound mail hosting are P1/P2 extensions. They must not be inferred from the current inbound prototype.

## Current repository state

The repository is a monorepo with Rust protocol/queue/delivery crates, a Go control plane, a Next.js dashboard, and PostgreSQL migrations. The current implementation is a local validation foundation, not a production service. In particular, control-plane state is still in memory, production identity integration is deferred, outbound delivery is disabled by default, DKIM signing and production object storage are not implemented, and cloud deployment is not selected.

## Architecture status vocabulary

- **Accepted** — already established by the repository or selected for the target architecture.
- **Provisional** — a reasoned recommendation pending benchmark, threat-model, business, or compliance validation.
- **Deferred** — intentionally outside Phase 01 and assigned to a later phase.
- **Open** — an unresolved question that can change a decision.

## Diagrams

Mermaid source files are in [`../../../diagrams/`](../../../diagrams/). The narrative embeds the same boundaries and names. The diagrams are source artifacts; rendering in CI is a Phase 05 concern.

## Remaining implementation

The architecture is implemented as the current design baseline, not as a
claim that every planned service exists. Cross-phase deferred work, reasons,
and application points are maintained in the
[phase implementation status](../../phase-status.md), especially the Phase 01
section.
