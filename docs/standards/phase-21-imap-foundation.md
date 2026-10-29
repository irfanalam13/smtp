# Phase 21 — IMAP Foundation

Status: **Mailbox storage contract implemented; IMAP service not yet complete**

RFC 9051 requires a session with non-authenticated, authenticated, and
selected states. Mailboxes are persistent named folders, while message
sequence numbers are session views and UIDs are stable mailbox identifiers.
The new mailbox storage migration provides folders, UID validity, raw messages,
flags, internal dates, and deletion timestamps.

The shared `MailboxStore` now provides transactional folder creation, stable
UID allocation, append, retrieval, flag updates, deletion marking, and
expunge primitives. The IMAP listener, TLS policy, authentication adapter,
selected-mailbox concurrency, and command parser remain to be implemented
against this store.
