# Phase 58 — Archive System

Status: **Permission-checked archive policy metadata implemented; cold storage and retrieval pending**

## Design contract

An Archive folder/label is a user organization action and does not imply
long-term archival, immutable retention, or cold storage. Retention periods,
legal holds, immutability, retrieval SLAs, residency, and deletion rules are
jurisdiction- and product-specific; no single mail RFC supplies those
requirements.

Archival storage must preserve verifiable message identity and integrity,
encryption/key lifecycle, an auditable restore path, and independent backup
and recovery evidence. Tier changes must not break mailbox visibility or quota
accounting.

## Implemented and remaining

Migration `000010` adds archive tier, retention deadline, and legal-hold
metadata. A permission-checked repository operation updates this policy; the
current message body remains in PostgreSQL `BYTEA`. No cold storage or
compliance guarantee is implied.

Still required: object-store selection, immutable-retention and legal
requirements, asynchronous tier/restore workers, integrity checks, key
management, lifecycle audit, retention-safe deletion, and measured retrieval
and recovery SLAs.

**Why this remains open:** The current operation stores policy metadata only;
it does not move bytes to an archive tier. Storage transitions require an
approved retention policy, verifiable object movement, key management, and
tested restore behavior before archived content can be relied upon.

## Acceptance criteria

- Archival state is durable and has a replayable transition history.
- Cold data can be verified and restored without silent corruption.
- Legal holds prevent purge and are authorization/audit protected.
- Backup, restore, retention, and deletion policies are independently tested.

## Sources

- [RFC 8621](https://www.rfc-editor.org/rfc/rfc8621.html) for mailbox
  synchronization semantics; cold archival policy is a product/compliance
  contract rather than a mail protocol feature.
