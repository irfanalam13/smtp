# Phase 24 — POP3 Server

Status: **Not implemented**

RFC 1939 defines authorization, transaction, and update states. USER/PASS,
STAT, LIST, RETR, DELE, RSET, NOOP, and QUIT require a mailbox snapshot and
transaction-scoped deletion marks. POP3 must not expose IMAP sequence state
directly; it consumes a stable mailbox retrieval view.
