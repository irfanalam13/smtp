# Phase 56 — Spam Folder Management

Status: **Permission-checked quarantine/release repository operations implemented; classifier and expiry worker pending**

## Design contract

Spam classification, quarantine, and user-visible Junk-folder organization are
separate concepts. The IMAP special-use role `\Junk` is standardized by RFC
6154; quarantine reason, release, review, and cleanup policy are platform
behavior. A message must not be silently destroyed merely because a classifier
labels it spam.

Quarantine decisions retain provenance, review deadlines, and release state.
Release restores a message through a transactional move and must not bypass
normal malware/authentication policy.

## Implemented and remaining

Migration `000010` adds mailbox-scoped quarantine entries, state, reason, and
review deadline. Permission-checked repository operations quarantine and
release messages transactionally with mailbox change journaling. Existing
default Junk special-use folders remain the user-facing folder primitive.

Still required: classifier decision integration, public review/reporting APIs,
scheduled expiration, retention/legal review, and abuse-resistant audit
controls. Repository moves and sync records are transactional.

**Why this remains open:** Repository operations do not classify messages or
schedule expiry, and no user-facing review workflow is connected. Classification
and cleanup must preserve review, audit, and legal-hold behavior rather than
turning a stored verdict into automatic irreversible deletion.

## Acceptance criteria

- Quarantined mail is isolated from normal delivery and search by explicit
  policy.
- Release and re-quarantine are idempotent and audited.
- Cleanup obeys configured retention and any legal hold.
- Classification confidence is not treated as proof of maliciousness.

## Sources

- [RFC 6154](https://www.rfc-editor.org/rfc/rfc6154.html)
- [RFC 9051](https://www.rfc-editor.org/rfc/rfc9051.html)
