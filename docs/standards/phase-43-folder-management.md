# Phase 43 — Folder Management

Status: **Nested storage primitives and default-folder provisioning implemented; protocol/API synchronization pending**

## Contract

Each mailbox has a root `INBOX`, `Sent`, `Drafts`, `Trash`, `Archive`, and
`Junk` folder set. IMAP special-use attributes follow [RFC 6154](https://www.rfc-editor.org/rfc/rfc6154):
`INBOX` is a reserved mailbox name, not a `\Inbox` special-use attribute.
User folders may be nested; folder names are stored as individual components,
not slash-delimited paths. Parent and child must belong to the same mailbox.

Folders carry stable IDs, UIDVALIDITY, monotonically increasing UIDNEXT, and
timestamps. A message UID is never reused in a folder, including after
expunge. Rename and move semantics must preserve the IMAP-visible UID
contract; if a move changes mailbox membership, the destination receives a
new UID. UIDVALIDITY values are allocated from a durable PostgreSQL sequence,
not wall-clock seconds, so folder recreation cannot immediately reuse a value.
Deleting a non-empty folder must either be rejected or use an
explicit recursive operation; silently cascading user data is not acceptable.

Synchronization requires a durable change journal with mailbox-scoped
monotonic cursors and bounded pagination. Folder metadata changes and message
mutations must become visible in a consistent order. The existing
process-local `SyncLog` is not a durable journal.

## Code and gaps

Migration `000008` adds parent references, same-mailbox constraints,
special-use metadata, UIDNEXT, and root/child uniqueness. Rust
`MailboxStore::ensure_default_folders` is transactional and repeatable;
`create_child_folder` persists nested components; append advances UIDNEXT
transactionally. Rename, move, safe deletion, folder ACLs, IMAP LIST/STATUS,
durable synchronization, and REST endpoints remain pending.

## Acceptance criteria

- Repeated provisioning creates exactly one of each default folder.
- Same names are allowed under different parents but not duplicated among
  siblings; a folder cannot parent itself or cross mailbox boundaries.
- Concurrent appends allocate unique, increasing UIDs without reuse.
- UIDVALIDITY changes only when a mailbox/folder is recreated, never on a
  restart or ordinary rename.
- Folder mutations and corresponding sync events commit atomically.
- Folder deletion tests cover non-empty folders, retention, and in-flight
  operations.
