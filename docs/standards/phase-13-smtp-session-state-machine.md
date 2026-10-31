# Phase 13 — SMTP Session State Machine

Status: **Implemented for the bounded submission profile**  
Verification date: **2026-10-29**

## Research basis

The session behavior follows [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321)
for command sequencing and transaction reset, and
[RFC 4954](https://www.rfc-editor.org/rfc/rfc4954) for SMTP AUTH:

- greeting precedes `EHLO`/`HELO`;
- `MAIL` precedes `RCPT`, and `RCPT` precedes `DATA`;
- `RSET` clears the current envelope but keeps the connection usable;
- `QUIT` closes the connection;
- `AUTH` is not valid during a mail transaction and cannot be repeated after
  successful authentication;
- AUTH mechanisms are advertised only when actually configured.

## State model

```text
Connected -> Greeted -> Authenticated -> MailFromAccepted
                         |                         |
                         +-------------------------+
                              -> RecipientsAccepted -> DATA -> Authenticated
```

`HELO`/`EHLO` establishes the greeting state and resets any incomplete
transaction. `RSET` returns to `Greeted`, or `Authenticated` when the session
has already authenticated. `QUIT` transitions to `Closed`.

## Authentication flow

The engine now exposes an injectable `Authenticator` boundary and implements
SASL `AUTH PLAIN`:

1. `EHLO` advertises `AUTH PLAIN` only when an authenticator is configured and
   the connection has completed TLS.
2. `AUTH PLAIN <initial-response>` is accepted, or the server sends `334` and
   reads one bounded continuation line.
3. The response is decoded and validated as the three-field PLAIN form.
4. Credentials are passed to the configured verifier without logging them.
5. Success returns `235` and enters `Authenticated`; invalid credentials
   return `535`, while verifier failures return `454`.
6. When `require_authentication` is enabled, unauthenticated `MAIL FROM` is
   rejected with `530`.

No default authenticator exists. The runnable service can use a
tenant-scoped PostgreSQL credential verifier. Tenant-qualified authcid values
use `<tenant UUID>:<username>`; usernames are normalized to lowercase ASCII.
TLS is mandatory for AUTH PLAIN. STARTTLS is advertised only when the server
has TLS material configured, and the session requires a fresh EHLO after
upgrading the transport.

## Validation boundary

The implementation keeps command lines, continuation lines, recipient counts,
message bytes, and connection counts bounded. It does not advertise
PIPELINING, SMTPUTF8, DSN, or other unsupported extensions. Credential
provisioning is tenant-scoped and exposed through the control-plane API;
authentication throttling is shared through PostgreSQL. Python `smtplib`
interoperability is exercised in the PostgreSQL-backed suite; a wider client
matrix remains to be verified.
