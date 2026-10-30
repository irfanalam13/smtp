# Phase 84 — Automation Engine

Status: **Bounded mailbox filter/rule primitives exist; general workflow service pending**

## Repository baseline

Mailbox filters use bounded typed conditions and deterministic priority
ordering; supported actions can run transactionally during PostgreSQL
append. The outbox has durable lease/ack/retry primitives. There is no
customer-facing workflow definition, scheduler, sequence engine, approval
state machine, or visual builder.

## Recommended workflow contract

Represent workflows as versioned, typed declarative graphs with a small
allowlisted set of triggers, conditions, and actions. No arbitrary scripts,
shell commands, unbounded expressions, or network calls from the evaluator.
Every execution has tenant/workflow/version identity, idempotency key,
correlation, durable state, bounded resource use, and auditable transitions.
External actions run asynchronously with retries and explicit
at-least-once semantics.

Scheduled work must be durable across restart, time-zone and daylight-saving
aware, cancellable, and protected against duplicate firing. Email sequences
must enforce consent, suppression, unsubscribe, sender policy, and
rate/volume controls. Approvals need explicit actors, timeout behavior, and
separation-of-duty policy. A visual builder is only a client for the same
validated workflow API, not an alternate execution engine.

## Remaining work and why

Define supported use cases, triggers, action semantics, execution limits,
scheduling rules, quotas, approval policy, versioning, cancellation,
observability, and failure recovery. Implement persistence, worker leasing,
idempotent action adapters, replay, dead letters, and visual workflow UX only
after event and authorization contracts are stable.

This remains open because append-time mail rules are not a general workflow
engine, and a durable outbox alone does not provide scheduling, state-machine
recovery, or safe external side effects. Email automation also has consent and
abuse implications that require product policy.

## Acceptance criteria

- Graph definitions validate against a bounded schema and are immutable per
  execution version.
- Duplicate triggers and retries cannot send duplicate emails or repeat
  irreversible actions.
- Cancellation, pause, replay, dead-letter, tenant fairness, and scheduler
  failover are tested.
- Sequences enforce unsubscribe/suppression, quotas, and sender authorization.
- Execution history is tenant-scoped, privacy-limited, and operationally
  observable.

## Research references

- [Phase 52 Mail Filters](phase-52-mail-filters.md)
- [Phase 53 Rule Engine](phase-53-rule-engine.md)
- [Phase 82 Event Bus and Messaging](phase-82-event-bus-messaging.md)
