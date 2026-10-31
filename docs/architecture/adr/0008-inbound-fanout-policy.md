# ADR 0008: Inbound fan-out is tenant-scoped and atomic

- Status: Accepted
- Date: 2026-10-30

## Context

An SMTP transaction can name multiple local recipients. Storing only one
mailbox copy rejects valid multi-mailbox delivery, while distributing one
message across unrelated tenants creates an implicit cross-tenant data-sharing
policy and complicates privacy, retention, and authorization.

## Decision

Inbound fan-out may target multiple active mailboxes only when all recipients
resolve to the same active tenant and verified local domains. A batch spanning
tenants is rejected with the same generic recipient error used for unknown or
inactive recipients. The complete set of mailbox copies and recipient links is
written in one PostgreSQL transaction; SMTP success follows commit only.

Inbound delivery remains opt-in. The SMTP profile has no caller-supplied
idempotency token, so the current versioned fingerprint treats byte-identical
sender, envelope, and raw content as a retry. This can deduplicate an
intentional identical resubmission and is documented as at-most-once insertion
for that fingerprint, not exactly-once delivery.

## Consequences

Same-tenant multi-mailbox transactions are supported with all-or-nothing
storage. Cross-tenant fan-out requires a future explicit product decision and
privacy review. Public inbound also requires configured ClamAV scanning and
shared PostgreSQL-backed IP/domain rate controls. SPF/DKIM/DMARC enforcement,
operational recovery for the inbound mailbox projection worker, retention, and
object storage remain prerequisites for a production hosted mailbox service.
