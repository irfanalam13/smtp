# Phase 27 — Threading and Conversations

## Research baseline

Thread identity follows RFC 5322 `Message-ID`, `In-Reply-To`, and `References`
headers. RFC 5256 describes IMAP threading algorithms; subject normalization
is only a fallback and must not override explicit references.

## Implemented foundation

`ThreadIndex` groups messages by known reference headers and exposes stable
conversation membership within its process lifetime. It supports insertion and
removal and keeps message relationships separate from mailbox folders.

## Production requirements

- Persist normalized message identifiers and conversation IDs.
- Handle malformed, duplicate, cyclic, and cross-mailbox references safely.
- Implement server-selected REFERENCES and ORDEREDSUBJECT algorithms.
- Rebuild threads after migration and support idempotent reprocessing.
- Apply tenant authorization before returning a conversation.

