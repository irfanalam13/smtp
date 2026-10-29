# Phase 63 — Mailbox Quotas

Status: **Transactional raw-message and draft quota enforcement plus authorized usage/policy operations implemented**

## Research and contract

Quota enforcement is a storage invariant, not a best-effort API check. The
mailbox row is updated transactionally on append and draft revision writes;
storage release is coupled to deletion. Attachment/object-store bytes,
replication overhead, and external backups are not currently included in this
mailbox byte count.

## Implementation and remaining

PostgreSQL tracks `used_bytes` and `quota_bytes`; append and immutable draft
revisions charge under row-level serialization, and Trash/expunge release
message bytes. The repository adds authorized usage reads and quota changes
that cannot lower the limit below current usage. Notification delivery,
warning history, storage-class accounting, and API routes remain pending.

## Acceptance criteria

- Concurrent writes cannot exceed a mailbox quota.
- Quota decreases below current use fail explicitly.
- Reported usage identifies the byte classes included and excluded.

## Sources

- [RFC 9208 — IMAP QUOTA Extension](https://www.rfc-editor.org/rfc/rfc9208.html)
- [RFC 2087 — IMAP QUOTA Extension](https://www.rfc-editor.org/rfc/rfc2087.html)
