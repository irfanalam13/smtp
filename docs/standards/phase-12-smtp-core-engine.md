# Phase 12 — SMTP Core Engine

Status: **Implemented for the bounded core profile**  
Verification date: **2026-10-29**

## Scope

Phase 12 provides a basic RFC 5321 SMTP server for local submission and
loopback/inbound acceptance, plus the existing bounded outbound SMTP client.
Ingress STARTTLS and authenticated submission are implemented under M01. This
phase does not claim public-production readiness: DNS policy, distributed
abuse controls, DKIM, DSN, and real external delivery remain separate
milestones.

## RFC 5321 profile

The server implements the core transaction:

```text
220 greeting
EHLO/HELO
MAIL FROM
RCPT TO (one or more)
DATA
message bytes terminated by <CRLF>.<CRLF>
250 acceptance after configured durable handoff
RSET / NOOP / QUIT
```

The implementation enforces:

- CRLF-terminated command and DATA lines.
- Configured command-line and message-size limits.
- Configured recipient and concurrent-connection limits.
- SMTP command sequencing and transaction reset.
- DATA dot transparency: one leading dot is removed only at the DATA boundary.
- A real `SIZE` EHLO capability derived from `max_message_bytes`.
- The RFC-valid null reverse-path (`MAIL FROM:<>`) for generated mail.
- Bounded `421`, `452`, `500`/`501`, `502`, `503`, `552`, and `250` outcomes
  for the implemented conditions.

The envelope remains separate from RFC 5322 message bytes. The server does
not authorize a sender using visible `From` headers.

## Sending path

Outbound SMTP is owned by `smtp-delivery`. It performs greeting handling,
EHLO/STARTTLS policy, envelope commands, DATA dot-stuffing, bounded reply
parsing, and final-response classification. The delivery configuration keeps
real external delivery disabled unless explicitly enabled with the required
destination-safety policy.

## Deliberate exclusions

- STARTTLS and AUTH capabilities are implemented under M01 and are advertised
  only when TLS material is configured and, for AUTH, the session is encrypted.
- `PIPELINING`, `DSN`, `SMTPUTF8`, and `REQUIRETLS` are not advertised.
- MIME decoding is not part of SMTP transport; raw message bytes are preserved
  for a later bounded parser.
- A `250` response means durable local acceptance or queue handoff, not inbox
  placement.

## Acceptance tests

The focused test suite covers command parsing, command ordering, null
reverse-path handling, advertised message limits, DATA dot-unstuffing,
oversized input, concurrent clients, and clean connection shutdown.
