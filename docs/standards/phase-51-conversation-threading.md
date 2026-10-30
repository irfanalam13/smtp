# Phase 51 — Conversation Threading

Status: **MIME-derived mailbox-scoped conversation linkage is persisted on append; full reconciliation pending**

## Research and contract

RFC 5322 defines `Message-ID`, `In-Reply-To`, and `References`; RFC 5256
defines IMAP `REFERENCES` and `ORDEREDSUBJECT` threading algorithms. RFC 8621
models JMAP email threads separately from messages. Explicit reference headers
are the primary relationship; subject normalization is a separate fallback
algorithm and must never override a valid reference chain.

Thread identity is scoped to a mailbox/tenant. Header identifiers are
normalized conservatively, malformed values are rejected by the core index,
and bounded reference lists prevent untrusted headers from consuming
unbounded memory. Late-arriving ancestors may join already-created provisional
threads; when references connect existing groups, the index merges them.

## Implemented and remaining

The in-process `ThreadIndex` validates and canonicalizes message IDs, groups
unresolved references provisionally, merges connected groups, and removes
message membership. PostgreSQL append parses MIME threading headers and stores
mailbox-scoped conversation linkage transactionally.

Still required: durable late-ancestor reconciliation/rebuild, RFC 5256
algorithm output for IMAP, idempotent replay, explicit duplicate-ID policy,
and concurrent/load testing. No IMAP threading capability is implied.

**Why this remains open:** Append-time linkage handles only part of threading.
Late-arriving ancestors, duplicate identifiers, and replay need durable,
deterministic reconciliation; the IMAP algorithm also requires protocol-level
integration and interoperability evidence.

## Acceptance criteria

- Cross-mailbox and cross-tenant references never expose or merge conversations.
- Duplicate, malformed, cyclic, missing, and late references produce
  deterministic, bounded results.
- Rebuilds are repeatable and do not change unrelated message identity.
- IMAP threading capabilities are advertised only for implemented algorithms.

## Sources

- [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322.html)
- [RFC 5256](https://www.rfc-editor.org/rfc/rfc5256.html)
- [RFC 8621](https://www.rfc-editor.org/rfc/rfc8621.html)
