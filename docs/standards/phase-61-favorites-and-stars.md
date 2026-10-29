# Phase 61 — Favorites and Stars

Status: **IMAP `\Flagged` storage, permission-checked mutation, and durable sync are implemented**

## Research and contract

IMAP `\Flagged` is the standard system flag for a message marked important or
interesting; RFC 9051 also defines flag persistence and `PERMANENTFLAGS`.
“Favorite” and “pin” are product concepts, not distinct IMAP system flags.
This phase uses `\Flagged` as the interoperable star and avoids a second,
divergent boolean for the same state. User-specific favorites would require a
separate principal-scoped model and are not implied by a shared mailbox flag.

## Implementation and remaining

The mailbox schema stores flags and the change journal captures flag updates.
The repository adds mailbox-authorized flag mutation and quota inspection.
Presentation, pinned-message ordering, per-user favorites, authenticated HTTP
routes, and IMAP command wiring remain pending.

## Acceptance criteria

- Flag mutations require mailbox write permission and emit a durable sync
  change.
- `\Flagged` has consistent meaning across IMAP and supported clients.
- Stars are not confused with spam classification or delivery priority.

## Sources

- [RFC 9051 — IMAP4rev2](https://www.rfc-editor.org/rfc/rfc9051.html)
- [RFC 6154 — IMAP LIST Extension for Special-Use Mailboxes](https://www.rfc-editor.org/rfc/rfc6154.html)
