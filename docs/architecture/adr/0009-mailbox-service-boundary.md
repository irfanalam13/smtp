# ADR 0009: Rust mailbox service owns mailbox data operations

- Status: Accepted for M08
- Date: 2026-10-30

## Context

Mailbox state and message bytes are PostgreSQL-backed Rust domain operations.
The Go API owns external authentication, API-key scope enforcement, request
context, and the public HTTP boundary. Implementing mailbox invariants again in
Go would create divergent authorization and quota behavior.

## Decision

The Rust `mailbox-service` owns mailbox provisioning, mailbox-scoped
authorization, folder/message/alias/permission/label/quota mutations, message
content reads, and their transactional database effects. The Go control plane
authenticates public requests and proxies them to the service with a short-lived
HMAC-signed principal bound to method, URI, and body digest. The append
idempotency key is included in that signed principal. Reads use
`mailbox:read`, writes use `mailbox:write`, and administrative operations use
`mailbox:manage`; PostgreSQL owner/membership/grant checks remain mandatory.

The service binds to loopback by default and rejects non-loopback binds without
TLS. A remote service URL must use HTTPS. The HMAC key is a shared deployment
secret and must not be logged or sent to external systems.

## Consequences

Public HTTP authentication and mailbox data invariants have a clear owner.
Mailbox content access can be audited at the data plane. The current storage
backend remains inline PostgreSQL `BYTEA`; this decision does not establish
object-storage scale, production identity lifecycle, or IMAP readiness.
