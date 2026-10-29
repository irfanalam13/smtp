# Phase 52 — Mail Filters

Status: **Bounded rules persist and execute transactionally on PostgreSQL append; SMTP delivery integration pending**

## Research and contract

RFC 5228 (Sieve) defines a deliberately constrained language evaluated during
final delivery; the base language excludes arbitrary loops, shell access, and
general-purpose code execution. RFC 5232 defines the `imap4flags` extension.
The platform's structured JSON rule model is application-specific and is not
claimed to be Sieve-compatible.

Rules have stable IDs, explicit enablement, ascending numeric priority, typed
conditions/actions, and an optional stop-after-match behavior. Incoming and
outgoing contexts are distinct policy surfaces. This implementation produces
an ordered action plan only; it does not move, reject, forward, or mutate mail.
Callers must validate mailbox/label ownership and apply effects transactionally.

## Implemented and remaining

Core provides a bounded, typed evaluator with deterministic ordering,
case-insensitive text predicates, attachment/size checks, and safe action
planning. PostgreSQL append evaluates enabled incoming/bidirectional rules
and applies supported folder, label, and flag actions in the message
transaction, recording executions atomically.

Still required: SMTP ingress/submission integration, authenticated HTTP CRUD,
Sieve interoperability if selected, per-tenant quotas, audit/observability,
duplicate-delivery semantics, and external delivery failure/retry tests. This
execution applies only to callers using the PostgreSQL append path.

## Acceptance criteria

- Evaluation has hard limits on rule count, expression depth, and action count.
- No arbitrary code, regex backtracking, network access, or shell execution is
  available to a rule.
- Invalid references fail closed and side effects are idempotent.
- Rule ordering and stop semantics are deterministic across retries.

## Sources

- [RFC 5228](https://www.rfc-editor.org/rfc/rfc5228.html)
- [RFC 5232](https://www.rfc-editor.org/rfc/rfc5232.html)
