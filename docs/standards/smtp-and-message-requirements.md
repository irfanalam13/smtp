# SMTP, message format, and MIME requirements

## SMTP session model

RFC 5321 sections 3–4 define a stateful dialogue: greeting, `EHLO`/`HELO`, envelope (`MAIL FROM`, one or more `RCPT TO`), `DATA`, and transaction reset/connection termination. A server must not accept commands valid only in a later state, must parse replies including multiline replies, and must return the appropriate 2xx/3xx/4xx/5xx class. The existing Rust session component owns this state machine.

The envelope sender and recipients are transport metadata. The RFC 5322 `From`, `To`, and `Cc` headers are message content and may differ. The platform must authorize the envelope identity and configured sending domain; it must not infer authorization from a visible header.

`DATA` is complete only after the SMTP end-of-data sequence. Dot transparency is applied on the wire and removed for stored content. Accepted content must retain required CRLF semantics and must not be reconstructed from selected headers. `250` means the platform accepted responsibility only after the Phase 01 durable commit.

## Required behavior

| Requirement                                                                  | Source                                | Owner             | Test                                 |
| ---------------------------------------------------------------------------- | ------------------------------------- | ----------------- | ------------------------------------ |
| Bound command line, message size, recipients, connections, and idle time     | RFC 5321 §4.5.3 plus platform policy  | SMTP session      | oversized input, timeout, exhaustion |
| Advertise only implemented EHLO extensions                                   | RFC 5321 §4.1.1.1; IANA SMTP registry | SMTP session      | capability contract                  |
| Accept `HELO` compatibility without falsely advertising modern extensions    | RFC 5321 §§4.1.1.1–4.1.1.2            | SMTP session      | HELO/EHLO tests                      |
| Parse multiline replies and enhanced status codes                            | RFC 5321 §4.2; RFC 3463               | transport/status  | reply parser tests                   |
| Dot-unstuff only at the DATA boundary                                        | RFC 5321 §4.5.2                       | message framing   | dot-stuffing tests                   |
| Do not pipeline unless command ordering and failure recovery are implemented | RFC 2920 §§3–4                        | session/transport | pipelining negative/positive tests   |
| Do not silently downgrade after an advertised required TLS failure           | RFC 3207 §4; Phase 01 policy          | TLS/transport     | downgrade tests                      |

## Extension support profile

| Extension    | RFC      | Initial decision                  | Advertisement/condition                                                                 |
| ------------ | -------- | --------------------------------- | --------------------------------------------------------------------------------------- |
| `SIZE`       | RFC 1870 | Required                          | Advertise a real maximum; reject declared or actual over-limit messages safely.         |
| `8BITMIME`   | RFC 6152 | Conditional                       | Advertise only when wire/content handling is correct; do not treat it as SMTPUTF8.      |
| `STARTTLS`   | RFC 3207 | Required before production        | Advertise only with a valid certificate/configuration and reset state after TLS.        |
| `AUTH`       | RFC 4954 | Required before public submission | Advertise only over protected transport and only supported mechanisms.                  |
| `PIPELINING` | RFC 2920 | Optional                          | Add after state-machine conformance tests; remote failures must not desynchronize work. |
| `DSN`        | RFC 3461 | Planned                           | Advertise only when parameter parsing and report generation are implemented.            |
| `SMTPUTF8`   | RFC 6531 | Optional first release            | Do not advertise until UTF-8 envelope/header/storage/DSN path is complete.              |
| `REQUIRETLS` | RFC 8689 | Optional                          | Do not accept or propagate a requirement that the delivery path cannot enforce.         |

## Response and delivery mapping

- 2xx positive completion: the current command or remote SMTP transaction succeeded. Remote `2xx` maps to `delivered_to_next_hop`, not inbox placement.
- 3xx positive intermediate: continue the required dialogue; it is not acceptance.
- 4xx transient negative completion: defer and retry within destination, tenant, and queue budgets.
- 5xx permanent negative completion: classify a terminal recipient/message failure; do not blindly retry.
- 521/556: apply RFC 7504 semantics where the response is valid; retain the enhanced code and remote diagnostic in restricted delivery metadata.
- Connection reset or timeout after sending a potentially accepted `DATA`: ambiguous outcome; record and retry under duplicate-risk policy.

Remote diagnostic text is useful for support but is untrusted input. Store bounded, sanitized values and do not expose credentials, full message content, or unnecessary remote personal data.

## Message and MIME representation

The system keeps separate:

1. SMTP envelope (`MAIL FROM`, `RCPT TO`, ESMTP parameters).
2. RFC 5322 headers/body bytes.
3. MIME structure and transfer encoding.
4. Internal tenant/message/delivery metadata.

RFC 5322 line and header grammar, RFC 2045/2046 body and multipart rules, RFC 2047 encoded words, and RFC 2231 parameters require bounded parsing. Header folding is syntax, not permission to accept control-character injection. Serialization must preserve required message content and should avoid rewriting signed headers after DKIM is introduced.

Malformed input is rejected or quarantined according to whether the failure occurs before responsibility transfer. MIME decoding must not recursively expand unbounded content, accept unsafe boundaries, or log raw parts. Attachments remain opaque content for the delivery product unless a later malware scanner integration explicitly inspects them.
