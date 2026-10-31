# Phase 45 — Message Metadata

Status: **Versioned bounded projections persist across queue, inbound, mailbox,
and API submission paths; rebuild and richer mailbox workflows pending**

## Contract

Keep immutable message bytes separate from mutable mailbox state. The
projection includes parsed `Message-ID`, `In-Reply-To`, `References`, subject,
sent/internal timestamps, conversation ID, flags, and user labels. Parsing
must tolerate malformed headers within strict byte/count bounds; parsed
metadata is a disposable projection and must be rebuildable from canonical
bytes.

`internal_date` is server-controlled and distinct from the untrusted message
`Date` header. IMAP system flags are stored with protocol semantics; custom
keywords and user labels are separate from system flags. Conversation grouping
is derived from validated message identifiers and reference chains, not
subject alone. A message may belong to more than one mailbox/folder, each with
independent flags and labels.

Metadata writes are authorized at mailbox scope and transactionally coupled
to a durable synchronization event. Search/thread indexes are asynchronous,
idempotent projections with rebuild cursors. No index result bypasses current
tenant and mailbox authorization.

## Code and gaps

Migration `000008` adds nullable header/conversation projections and a
message-label relation while retaining the existing system-flag array and
keywords table. Migration `000020` adds schema-versioned bounded parsed
metadata to queue, inbound, and mailbox messages. SMTP/API submissions and
mailbox append now persist search/render metadata while retaining exact raw
bytes. The PostgreSQL append path derives a mailbox-scoped conversation link
from MIME threading headers. Read/unread and priority APIs, thread
reconciliation, label APIs, projection rebuilds, and asynchronous
search-index population remain pending.

**Why this remains open:** A few append-time fields do not amount to a complete
metadata pipeline. Projections must be derived reliably from canonical bytes,
updated transactionally or replayably, and synchronized with authorized
mailbox changes; otherwise search and clients can observe incomplete or stale
state.

## Acceptance criteria

- Malformed and duplicate message identifiers cannot create cross-tenant or
  cyclic conversation relationships.
- Projection rebuild produces equivalent metadata without modifying raw
  message bytes.
- System flags, keywords, and labels retain their distinct semantics through
  concurrent updates and synchronization.
- Internal date cannot be overwritten by an untrusted header.
- Search/thread results are mailbox-authorized and consistent after replay.

## References

- [RFC 5322 — Internet Message Format](https://www.rfc-editor.org/rfc/rfc5322)
- [RFC 9051 — IMAP4rev2](https://www.rfc-editor.org/rfc/rfc9051)
- [RFC 5256 — IMAP SORT and THREAD](https://www.rfc-editor.org/rfc/rfc5256)
