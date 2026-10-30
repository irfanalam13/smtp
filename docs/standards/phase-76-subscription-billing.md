# Phase 76 — Subscription and Billing

Status: **Billing boundary documented; plans, metering, provider integration, and customer workflows pending**

## Repository baseline

The architecture assigns billing references and policy to the control plane
and usage facts to asynchronous analytics. Current usage/readiness data is
not a durable billing ledger. There are no subscription, invoice, payment,
plan-entitlement, or usage-rating workflows, and no payment provider has been
selected.

## Recommended billing boundary

Keep plan/entitlement decisions and customer-visible billing state in the
control plane. Store a provider-neutral internal account/subscription ID and
provider references; do not store payment-card data. Use the provider's
hosted/secure payment collection and verify signed provider callbacks.

Record immutable, idempotent usage facts from authoritative events or
transactions, with tenant, meter, quantity/unit, event time, source ID, and
deduplication key. Aggregate/rate asynchronously and reconcile against source
events. Plan changes must have explicit effective times and proration
semantics. Quotas and abuse/safety limits must remain enforced locally even
when billing is unavailable. Billing, payment, and analytics systems must
never block SMTP acceptance or delivery.

## Remaining work and why

Choose a payment provider after product, geography, tax, currency, and
merchant-of-record requirements are known. Define plans, entitlements, trial
rules, metered units, rounding, overage/denial policy, refunds, invoices,
tax, dunning, cancellation, retention, reconciliation, and support operations.
Then build a provider adapter, durable usage ledger, webhook verification,
idempotent handlers, and sandbox/contract tests.

This remains open because the repository has no billing product policy or
provider contract. Billing correctness has financial and legal consequences;
guessing rates, units, tax behavior, or suspension policy in code would be
unsafe. Mail safety limits cannot depend on an external billing callback.

## Acceptance criteria

- Usage is derived from authoritative, replayable facts and duplicate events
  cannot double-charge.
- Plan changes, effective dates, overages, and cancellation behavior are
  deterministic and audited.
- Provider callbacks validate signatures/timestamps, are idempotent, and
  tolerate retries/out-of-order delivery.
- Provider outage does not bypass local quotas or block mail acceptance.
- Reconciliation detects missing, duplicate, or inconsistent usage and has an
  operator recovery procedure.

## Research references

- [Repository ownership and usage-data boundaries](../architecture/phase-01/data-and-deployment.md)
- [Control/data-plane separation](../architecture/adr/phase-01/0011-control-data-plane.md)
- [Transactional outbox proposal](../architecture/adr/phase-01/0013-event-architecture.md)
