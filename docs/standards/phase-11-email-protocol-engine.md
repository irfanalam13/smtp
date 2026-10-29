# Phase 11 — Email Protocol Engine Foundation

Status: **Research complete; foundation slice implemented**  
Verification date: **2026-10-29**  
Scope: the protocol boundaries and message lifecycle that the SMTP engine,
queue, delivery worker, and future mailbox protocols share.

## Research conclusions

Email is not one protocol or one representation. The implementation must keep
these layers separate:

1. **SMTP envelope** — the reverse path and one or more forward paths exchanged
   by `MAIL FROM` and `RCPT TO`. This controls transport and authorization.
2. **Internet message** — RFC 5322 header and body bytes received after `DATA`.
   Visible `From`, `To`, and `Cc` headers are content and do not replace the
   SMTP envelope.
3. **MIME representation** — content types, multipart boundaries, transfer
   encodings, and encoded headers. MIME is parsed only at a bounded,
   explicitly-owned boundary; it is not reconstructed from selected headers.
4. **Internal lifecycle metadata** — tenant, message ID, idempotency,
   attempts, leases, delivery status, and audit information.

The lifecycle is:

```text
MUA -> submission/session -> durable acceptance -> queue
    -> destination resolution and SMTP relay -> recipient MTA/MDA
    -> mailbox storage -> IMAP/POP3 client retrieval
```

The queue’s durable acceptance point is the boundary at which the service may
return SMTP `250`. A remote `250` means acceptance by the next hop, not inbox
placement.

## Normative source map

| Area | Primary source | Engineering consequence |
| --- | --- | --- |
| SMTP dialogue, commands, replies, limits, and relay | [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321) | Keep the session stateful; enforce command ordering, bounded input, and 2xx/3xx/4xx/5xx semantics. |
| Internet message syntax | [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322) | Preserve the received message bytes; envelope and visible headers remain separate. |
| MIME body and transfer encoding | [RFC 2045](https://www.rfc-editor.org/rfc/rfc2045), [RFC 2046](https://www.rfc-editor.org/rfc/rfc2046) | Treat MIME as bounded content parsing, not as transport routing. |
| Encoded header values | [RFC 2047](https://www.rfc-editor.org/rfc/rfc2047) | Decode only for presentation; do not use untrusted display headers for authorization. |
| Submission profile | [RFC 6409](https://www.rfc-editor.org/rfc/rfc6409) | Submission authentication and policy belong at the MSA boundary. |
| TLS upgrade | [RFC 3207](https://www.rfc-editor.org/rfc/rfc3207) | STARTTLS resets SMTP transaction state and must not silently downgrade a required TLS policy. |
| Enhanced status codes | [RFC 3463](https://www.rfc-editor.org/rfc/rfc3463) | Preserve bounded remote diagnostics and map transient/permanent failures to retry policy. |
| Mailbox access (future scope) | [RFC 9051](https://www.rfc-editor.org/rfc/rfc9051) | IMAP is a storage/access milestone, not part of the SMTP acceptance path. |

## Phase 11 implementation boundary

This phase implements the shared foundation, not full protocol conformance:

- Queue envelope validation now accepts the RFC-defined null reverse-path for
  generated mail and rejects non-ASCII, whitespace, control-character,
  malformed, and overlong forward paths.
- Raw message bytes remain opaque to the queue and are size-bounded by
  `QueueConfig`; the queue does not confuse RFC 5322 headers with envelope
  authorization.
- Existing SMTP session framing remains responsible for CRLF, DATA completion,
  and dot transparency.

The null reverse-path is represented internally as an empty sender string. It
is a transport value only; serializers at the SMTP wire boundary must emit
`<>`.

## Deferred work and entry criteria

| Work | Why not in this slice | Entry criteria |
| --- | --- | --- |
| Complete SMTP command conformance | Requires a full state/reply fixture matrix and interoperability tests | Add RFC 5321 command, reply, line-limit, DATA, and reset fixtures. |
| MIME parser | Recursive parsing and resource limits need a dedicated API and abuse tests | Define supported media types, nesting/decoded-size limits, and error policy. |
| SMTPUTF8/IDNA | Changes address, header, storage, and delivery contracts together | Approve internationalized-address product profile and end-to-end Unicode storage. |
| DKIM/ARC/DMARC | Signing and verification depend on canonicalization and DNS/key lifecycle | Implement DNS/key management and immutable message signing boundaries. |
| DSN/bounce processing | Requires delivery status persistence and recipient-level outcomes | Define status schema, retry classification, and feedback/suppression policy. |
| IMAP/POP3 | Retrieval is a separate mailbox product boundary | Approve hosted-mailbox scope and mailbox storage model. |

## Validation

The queue model tests cover valid envelope paths, the null reverse-path, and
injection/encoding rejection. Broader protocol conformance remains tracked in
[`smtp-and-message-requirements.md`](./smtp-and-message-requirements.md) and
must not be inferred from these unit tests.
