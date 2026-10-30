# Phase 54 — Labels and Categories

Status: **Permission-checked label creation/assignment and sync journaling implemented; public APIs pending**

## Research and contract

RFC 8621 defines JMAP keywords and mailbox membership; RFC 6154 defines
standard IMAP special-use mailbox attributes such as `\Sent`, `\Drafts`,
`\Trash`, and `\Junk`. Neither defines Gmail-style colored labels or
categories. Those remain product-level metadata and must not be advertised as
an IMAP standard.

Static labels/categories may be assigned many-to-many to messages. Smart
labels are virtual, query-backed views and must not duplicate message bytes or
be manually assigned. Special-use folders remain distinct from labels.

## Implemented and remaining

Migration `000010` adds mailbox-owned labels with bounded color/query metadata
and mailbox-consistent message assignments; existing text labels are migrated
into the new model. Repository methods permission-check label creation and
assignment, while message mutations update the durable mailbox change journal.
The model distinguishes labels, categories, tags, and smart labels.

Still required: update/delete and bulk APIs, virtual query validation/execution,
cache invalidation, IMAP/JMAP mapping, and migration/reconciliation tests at
production data volume.

**Why this remains open:** The repository implements only a subset of the
label lifecycle and these operations are not a complete public client
contract. Smart-label execution and protocol mapping also need bounded,
validated semantics so they cannot return stale or unauthorized messages.

## Acceptance criteria

- Assignment cannot cross mailbox boundaries.
- Smart labels are derived views, not mutable assignment rows.
- Folder roles, system flags, labels, and categories have separate semantics.
- Search results and synchronization reflect committed label changes.

## Sources

- [RFC 8621](https://www.rfc-editor.org/rfc/rfc8621.html)
- [RFC 6154](https://www.rfc-editor.org/rfc/rfc6154.html)
