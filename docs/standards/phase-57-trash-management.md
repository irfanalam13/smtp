# Phase 57 — Trash Management

Status: **Permission-checked transactional Trash move/restore/purge methods implemented; scheduled cleanup pending**

## Design contract

RFC 9051 defines IMAP `\Deleted` flags and expunge behavior; RFC 6851 defines
the IMAP MOVE extension. These protocol operations do not define a universal
consumer recovery window. User-facing Trash is a separate move to the
`\Trash` special-use mailbox, and retention/purge timing is an explicit
product and compliance policy.

Restoration returns a message to its prior folder with a newly allocated UID;
UIDs are never reused. Permanent expunge, user Trash deletion, automated
expiration, and legal hold must remain distinct operations.

## Implemented and remaining

Migration `000010` stores mailbox-scoped original-folder and purge-deadline
metadata for recoverable trash moves. Repository methods move and restore
messages, allocate a new UID on restore, write change-journal entries, and
refuse purging messages under legal hold. Existing `deleted_at` remains the
protocol/expunge marker and is not reinterpreted as a recovery window.

Still required: scheduled purge worker, configurable retention operations,
protocol/API integration, and disaster-recovery/load tests. No worker currently
invokes purge on a schedule.

## Acceptance criteria

- Restore and purge are mutually safe, authorized, and idempotent.
- UIDs are allocated monotonically on restore/move.
- Expiration never overrides an active legal hold.
- Quota and synchronization updates commit atomically with message movement.

## Sources

- [RFC 9051](https://www.rfc-editor.org/rfc/rfc9051.html)
- [RFC 6851](https://www.rfc-editor.org/rfc/rfc6851.html)
- [RFC 6154](https://www.rfc-editor.org/rfc/rfc6154.html)
