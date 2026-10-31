# Phase 15 — SMTP Extensions (ESMTP)

Status: **Bounded SMTP/ESMTP profile implemented and tested;**
**optional extensions remain gated**
Verification date: **2026-10-30**

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

The server always advertises `SIZE <global-configured-limit>` after EHLO.
`MAIL FROM` accepts one decimal `SIZE=<decimal>` parameter and rejects a
declaration above the effective limit before `DATA`. Actual de-transparencied
DATA bytes, including CRLF, are independently bounded while streaming.
Authenticated submission uses the minimum of the global, tenant, and verified
sender-domain limits. Tenant limits are stored in `tenants.max_smtp_message_bytes`
(default 10 MiB); an optional `domains.max_smtp_message_bytes` may lower that
tenant cap. The global limit is configured with `SMTP_MAX_MESSAGE_BYTES` and
is capped at 10 MiB to match queue/inbound storage and the canonical MIME
parser's absolute raw-message limit.

`8BITMIME` is implemented as an opt-in capability (`Config::enable_8bitmime`).
When enabled with `SMTP_ENABLE_8BITMIME=true`, EHLO advertises it and `MAIL
FROM` accepts one `BODY=8BITMIME` or `BODY=7BIT` parameter. 8-bit octets are
accepted only in the message body when explicitly declared; message headers
remain ASCII until SMTPUTF8 is designed end to end. Raw message octets are
preserved through the current queue/inbound boundary and remain subject to
line and effective total-size limits. The default remains disabled.

The command parser enforces visible ASCII command syntax, strict CRLF,
bounded dot-atom mailbox paths, and deterministic rejection of duplicate,
unknown, empty, non-decimal, or malformed `MAIL FROM` parameters. Extended
parameters require EHLO; HELO does not enable ESMTP extensions. SMTP command
lines are capped at 512 octets and DATA lines at 1,000 octets, both including
CRLF. DATA dot transparency is removed for any leading dot, and each
transaction is reset after success or recoverable DATA rejection. The
configured server name is validated against reply-line injection.

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

## Acceptance evidence and profile boundary

The TCP and parser suites cover advertised capabilities, invalid and duplicate
parameters, HELO/EHLO extension negotiation, declared and actual size limits,
tenant/domain limit lookup, 7-bit rejection, opt-in 8BITMIME preservation,
ASCII-only headers, dot transparency, strict CRLF, and transaction recovery.
The parser has bounded adversarial-input coverage. The PostgreSQL integration
suite exercises persisted tenant/domain size limits.

PIPELINING, CHUNKING/BINARYMIME, SMTPUTF8, and DSN remain deliberately
unadvertised. SMTPUTF8 requires end-to-end Unicode storage and outbound
handling; DSN requires durable recipient-level outcomes; PIPELINING and
CHUNKING require separate ordered-response/backpressure implementations.
