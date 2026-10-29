# Phase 22 — IMAP Core Commands

Status: **Not implemented**

The planned command surface is LOGIN, SELECT, FETCH, STORE, SEARCH, COPY,
MOVE, APPEND, DELETE, and EXPUNGE. Each command must enforce IMAP state,
literal-size limits, tagged/untagged response grammar, UID versus sequence
semantics, and transactional mutation behavior.

Implementation is blocked on the Phase 21 session/storage adapter and a
credential/TLS policy; no command is advertised as supported yet.
