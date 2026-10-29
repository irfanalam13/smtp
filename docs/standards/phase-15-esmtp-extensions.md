# Phase 15 — SMTP Extensions (ESMTP)

Status: **Capability-gated foundation implemented; full extension profile
remains in progress**  
Verification date: **2026-10-29**

## Research basis

- [RFC 1869](https://www.rfc-editor.org/rfc/rfc1869) defines EHLO extension
  negotiation and the rule that a server advertises only extensions it
  supports.
- [RFC 1870](https://www.rfc-editor.org/rfc/rfc1870) defines `SIZE` and the
  `MAIL FROM` `SIZE=` parameter.
- [RFC 6152](https://www.rfc-editor.org/rfc/rfc6152) defines `8BITMIME` and
  the `BODY=7BIT|8BITMIME` parameter.
- [RFC 2920](https://www.rfc-editor.org/rfc/rfc2920) defines `PIPELINING`;
  advertising it requires safe ordered processing and recovery.
- [RFC 3030](https://www.rfc-editor.org/rfc/rfc3030) defines `CHUNKING` and
  exact-byte `BDAT` framing.
- [RFC 6531](https://www.rfc-editor.org/rfc/rfc6531) defines `SMTPUTF8`,
  including internationalized envelope and message requirements.
- [RFC 3461](https://www.rfc-editor.org/rfc/rfc3461) defines DSN parameters,
  recipient notification policy, and delivery-status metadata.

## Implemented profile

The server always advertises `SIZE <configured-limit>`. `MAIL FROM` accepts
the bounded `SIZE=<decimal>` parameter and rejects a declaration above the
configured maximum before `DATA`. Actual DATA bytes remain independently
bounded.

`8BITMIME` is implemented as an opt-in capability (`Config::enable_8bitmime`).
When enabled, EHLO advertises it and `MAIL FROM` accepts `BODY=8BITMIME` or
`BODY=7BIT`. The current message storage path preserves raw octets and still
enforces line and total-size limits. The default remains disabled for
backwards-compatible conservative operation.

The parser rejects unknown `MAIL FROM` parameters rather than silently
accepting or forwarding them.

## Deliberately unadvertised

- **PIPELINING:** the current DATA transition requires the client to wait for
  `354`; advertising PIPELINING without a complete response-order and
  recovery matrix would be unsafe.
- **CHUNKING:** `BDAT` exact-byte framing and `LAST` transaction handling are
  not implemented.
- **SMTPUTF8:** address, header, storage, and outbound delivery are still
  ASCII-profiled.
- **DSN:** `NOTIFY`, `ORCPT`, `RET`, and `ENVID` need recipient-level delivery
  persistence and DSN generation.

These capabilities must not be added to EHLO until their complete wire,
storage, and delivery semantics are implemented.

## Acceptance criteria

The remaining Phase 15 work is complete when each advertised extension has
positive and negative parser tests, oversize/invalid-parameter tests,
transaction-reset coverage, and end-to-end interoperability fixtures. This
slice establishes the capability gate and safe `SIZE`/`8BITMIME` foundation
without making unsupported extensions appear available.
