# Phase 67 — Synchronization Engine

Status: **Permission-checked incremental mailbox change reads implemented; IMAP multi-device protocol engine remains pending**

## Research and contract

IMAP synchronization is based on mailbox UIDs, UIDVALIDITY, flags, and
mod-sequences. RFC 7162 CONDSTORE/QRESYNC reduces full mailbox scans but has
protocol-specific invariants. The internal change journal is an adapter
substrate; it does not claim IMAP synchronization conformance.

## Implementation and remaining

PostgreSQL records message, flag, and folder changes and provides bounded,
mailbox-authorized cursor reads. Auxiliary metadata changes are journaled by
the current migration. Cursor retention/reset, UIDVALIDITY reset behavior,
CONDSTORE/QRESYNC protocol support, backpressure, load benchmarks, and real
client interoperability remain pending.

## Acceptance criteria

- Changes are ordered per mailbox and reads have stable bounded pagination.
- Expired cursors produce an explicit resync requirement, not silent gaps.
- Multiple devices converge after concurrent moves and flag updates.

## Sources

- [RFC 9051 — IMAP4rev2](https://www.rfc-editor.org/rfc/rfc9051.html)
- [RFC 7162 — IMAP CONDSTORE and QRESYNC](https://www.rfc-editor.org/rfc/rfc7162.html)
