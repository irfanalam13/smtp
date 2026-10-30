# Phase 62 — Read Status Synchronization

Status: **Message and auxiliary metadata journaling plus incremental PostgreSQL reads implemented; protocol adapters remain pending**

## Research and contract

IMAP `\Seen` and `\Flagged` are mailbox message flags. RFC 7162 QRESYNC
builds efficient resynchronization on mod-sequences and known UIDs; a generic
monotonic event cursor is useful but is not itself QRESYNC. Change cursors are
mailbox-scoped, bounded, and must never be treated as authorization tokens.

## Implementation and remaining

The repository exposes permission-checked incremental changes. Database
triggers journal message insert/delete/move/flag updates. Migration `000011`
also journals label assignments, draft revisions, and custom-property
updates. Cursor expiry/full-resync, retention/compaction, mod-sequences, IMAP
QRESYNC/CONDSTORE, event consumers, and authenticated web/mobile APIs remain
pending.

**Why this remains open:** The journal records committed changes but does not
yet give clients a complete recovery contract when cursors expire, nor does it
provide protocol adapters that make devices converge. Retention and
compaction also need explicit cursor invalidation behavior to avoid silent
gaps.

## Acceptance criteria

- Every committed user-visible mutation is recoverable from a cursor or causes
  an explicit full-resync response when history has expired.
- Read-only users can sync but cannot mutate state.
- Cursor reads are bounded and strictly mailbox-authorized.

## Sources

- [RFC 9051 — IMAP4rev2](https://www.rfc-editor.org/rfc/rfc9051.html)
- [RFC 7162 — IMAP CONDSTORE and QRESYNC](https://www.rfc-editor.org/rfc/rfc7162.html)
- [RFC 8621 — JMAP Core and Mail](https://www.rfc-editor.org/rfc/rfc8621.html)
