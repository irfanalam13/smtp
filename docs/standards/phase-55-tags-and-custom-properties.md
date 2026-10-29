# Phase 55 — Tags and Custom Properties

Status: **Mailbox-owned tag assignments and bounded property storage schema implemented; property mutation API pending**

## Design contract

User-defined tags reuse mailbox-owned label assignments with a distinct `tag`
kind. Message custom properties are a bounded JSON object, not message headers
or trusted identity data. Keys and values require API-level schema, size,
privacy, and authorization controls. User metadata must never change canonical
RFC message bytes.

## Implemented and remaining

Migration `000010` adds mailbox-scoped tags and a size-limited JSONB custom
property projection with a GIN index. Cross-mailbox assignment is constrained
by composite foreign keys. Tags use the permission-checked label assignment
repository operation.

Still required: property mutation and authenticated HTTP APIs, property key
schema and reserved-key policy, audit and export/deletion semantics, selective
indexing, query cost controls, and privacy/retention review. The SQL index does
not by itself provide a stable public search API.

## Acceptance criteria

- Untrusted properties are never interpreted as authorization or protocol
  state.
- Size and query limits are enforced before persistence and in storage.
- Tenant isolation is tested for writes, filters, and search.
- Deletion/retention behavior covers properties and tag associations.

## Sources

- [RFC 8621](https://www.rfc-editor.org/rfc/rfc8621.html) provides a standard
  synchronized mail model; arbitrary custom properties are an application
  extension.
- [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322.html) defines canonical
  message headers, which these annotations do not modify.
