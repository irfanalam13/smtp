# Phase 53 — Rule Engine

Status: **Bounded evaluator and transactional append-time actions implemented; async workers/adapters pending**

## Design contract

The engine accepts a typed expression tree rather than user-provided code.
Boolean composition and a small set of explicit predicates are evaluated
against a bounded message projection. Rules run in deterministic priority/ID
order; the planner returns actions and matched rule IDs without executing side
effects. The caller owns authorization, transaction boundaries, retries, and
idempotency.

The engine rejects oversized/deep expressions and oversized rule/action
collections. It does not expose a regular-expression engine, loops, reflection,
filesystem access, shell commands, or network access. New predicates and
actions require a reviewed type and explicit authorization behavior.

## Implemented and remaining

The core module defines structured conditions, predicates, actions, validation,
evaluation, and stop-processing semantics with unit coverage. The PostgreSQL
append transaction evaluates stored rules and applies supported actions. The
mailbox outbox has durable lease/ack/retry operations.

Still required: broker consumer and delivery-pipeline adapter, retries and
dead-letter operations, per-tenant concurrency/fairness, execution audit,
versioned stored-rule schema migration, and load/failure testing. The outbox
primitive is not a running worker or autonomous workflow platform.

## Acceptance criteria

- Evaluation is deterministic, bounded, and side-effect-free.
- Persisted rule versions can be replayed and audited.
- Side effects are permission-checked and idempotent.
- Failure/retry behavior is documented and tested before enabling rules.

## Sources

- [RFC 5228](https://www.rfc-editor.org/rfc/rfc5228.html) for the constrained
  email-filtering design precedent.
- [RFC 8621](https://www.rfc-editor.org/rfc/rfc8621.html) for synchronized
  mail data and state changes.
