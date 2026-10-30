# Phase 64 — Retention Policies

Status: **Per-message retention/legal-hold metadata and guarded Trash purge implemented; mailbox policy scheduling pending**

## Research and contract

Email RFCs do not define a universal retention duration. Retention and legal
hold require explicit tenant policy, jurisdictional review, auditable
authorization, and deletion semantics. An expiration worker must be bounded,
restartable, and must never override an active hold.

## Implementation and remaining

The workflow schema stores per-message retention deadlines and legal holds;
Trash purge skips held messages. A periodic worker, mailbox-level policy
management, audit trail for hold changes, deletion attestations, and
jurisdiction-specific configuration are not implemented.

**Why this remains open:** Metadata and a guarded purge operation do not apply
a retention policy automatically. The policy owner, jurisdictional rules, and
audited transitions must be settled before a scheduled worker can safely
delete customer data; no automatic expiration is enabled.

## Acceptance criteria

- Purge is idempotent and hold-aware.
- Policy changes are authorized and auditable.
- Expiration is tested for retries, races with restoration, and recovery.

## Sources

- [RFC 5322 — Internet Message Format](https://www.rfc-editor.org/rfc/rfc5322.html)
- Product retention and legal-hold policy must be approved independently of
  protocol conformance.
