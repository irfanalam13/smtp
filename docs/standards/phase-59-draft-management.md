# Phase 59 — Draft Management

Status: **Permission-checked immutable draft revision save with optimistic version/quota checks implemented; public APIs pending**

## Design contract

Drafts are mailbox messages associated with the `\Drafts` special-use role,
not an outbound delivery success. Draft autosave must be idempotent, ordered,
quota-accounted, and isolated from concurrent editors. Version history must
not mutate the currently saved draft or rewrite a message after submission.

The platform stores full immutable revisions in PostgreSQL for this bounded
foundation and charges their bytes against mailbox quota. A future object
store adapter may replace the payload column only with equivalent atomic
visibility, retention, quota, and recovery guarantees.

## Implemented and remaining

Migration `000010` adds draft metadata and immutable payload revisions. A
database trigger charges and releases revision bytes transactionally against
mailbox quota. The repository save operation checks the expected version,
authorization, size, and quota atomically.

Still required: authenticated autosave/version/restore APIs, cross-device
protocol synchronization, draft expiry policy, blob storage integration, and
production concurrency/quota tests. No autosave endpoint is wired yet.

## Acceptance criteria

- Each accepted revision is immutable and ordered per draft.
- Concurrent stale saves cannot silently overwrite a newer revision.
- Quota charge/release is atomic with revision creation/retention.
- Draft save and submission have explicit, idempotent state transitions.

## Sources

- [RFC 6154](https://www.rfc-editor.org/rfc/rfc6154.html) defines the IMAP
  `\Drafts` special-use role.
- [RFC 8621](https://www.rfc-editor.org/rfc/rfc8621.html) defines synchronized
  mail objects and update semantics.
