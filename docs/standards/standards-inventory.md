# RFC inventory and relationship map

The tables in this document are the authoritative row-level inventory for
Phase 02. This document explains the important relationships and
implementation interpretation. All URLs point to official RFC Editor text or
official IETF/IANA records and were checked on 2026-10-28.

## Core SMTP and message format

| RFC                                                 | Title                           | Category/status | Relationship and use                                                                       |
| --------------------------------------------------- | ------------------------------- | --------------- | ------------------------------------------------------------------------------------------ |
| [5321](https://www.rfc-editor.org/rfc/rfc5321.html) | Simple Mail Transfer Protocol   | Standards Track | Obsoletes 2821; updates 1123. Core SMTP state, commands, replies, routing, and transport.  |
| [5322](https://www.rfc-editor.org/rfc/rfc5322.html) | Internet Message Format         | Standards Track | Obsoletes 2822; updates 4021. RFC 5322 message syntax is distinct from the SMTP envelope.  |
| [2045](https://www.rfc-editor.org/rfc/rfc2045.html) | MIME Part One                   | Standards Track | MIME syntax, transfer encoding, and headers; updated by 2231 and 6532.                     |
| [2046](https://www.rfc-editor.org/rfc/rfc2046.html) | MIME Part Two                   | Standards Track | Media types and multipart structures.                                                      |
| [2047](https://www.rfc-editor.org/rfc/rfc2047.html) | MIME Part Three                 | Standards Track | Encoded words for applicable structured/unstructured headers.                              |
| [2231](https://www.rfc-editor.org/rfc/rfc2231.html) | MIME Parameter Value Extensions | Standards Track | Parameter continuations and non-ASCII values; interoperability requires fallback handling. |
| [6152](https://www.rfc-editor.org/rfc/rfc6152.html) | 8BITMIME                        | Standards Track | Obsoletes 1652; permits 8-bit body transport only when advertised.                         |
| [1870](https://www.rfc-editor.org/rfc/rfc1870.html) | SIZE                            | Standards Track | Message-size declaration and server limit advertisement.                                   |
| [2920](https://www.rfc-editor.org/rfc/rfc2920.html) | PIPELINING                      | Standards Track | Obsoletes 1854; command batching with strict response/state rules.                         |

## Internationalization and status

| RFC                                                 | Title                                                           | Category/status | Relationship and use                                                                           |
| --------------------------------------------------- | --------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------------- |
| [6531](https://www.rfc-editor.org/rfc/rfc6531.html) | SMTP Extension for Internationalized Email                      | Standards Track | Obsoletes 5336; SMTPUTF8 capability and envelope rules.                                        |
| [6532](https://www.rfc-editor.org/rfc/rfc6532.html) | Internationalized Email Headers                                 | Standards Track | Obsoletes 5335; updates 2045; UTF-8 header requirements.                                       |
| [6533](https://www.rfc-editor.org/rfc/rfc6533.html) | Internationalized Delivery Status and Disposition Notifications | Standards Track | Obsoletes 5337; updates 3461, 3464, 3798, 6522.                                                |
| [3461](https://www.rfc-editor.org/rfc/rfc3461.html) | Delivery Status Notifications                                   | Standards Track | SMTP DSN extension; updated by 6533.                                                           |
| [3463](https://www.rfc-editor.org/rfc/rfc3463.html) | Enhanced Mail System Status Codes                               | Standards Track | Structured diagnostic code semantics.                                                          |
| [3464](https://www.rfc-editor.org/rfc/rfc3464.html) | Delivery Status Notifications                                   | Standards Track | Message/delivery report format; updated by 6533.                                               |
| [3462](https://www.rfc-editor.org/rfc/rfc3462.html) | Multipart/Report Content Type                                   | Standards Track | Reporting media type foundation; complemented by 6522 and updated internationalized reporting. |
| [6522](https://www.rfc-editor.org/rfc/rfc6522.html) | Multipart/Report Media Type                                     | Standards Track | Reporting media type; updated by 6533.                                                         |
| [7504](https://www.rfc-editor.org/rfc/rfc7504.html) | SMTP 521 and 556 Reply Codes                                    | Standards Track | Defines additional permanent reply-code uses.                                                  |
| [7505](https://www.rfc-editor.org/rfc/rfc7505.html) | Null MX                                                         | Standards Track | Explicitly declares that a domain accepts no mail.                                             |

## TLS, submission, and authentication

| RFC                                                 | Title                                           | Category/status       | Relationship and use                                                                           |
| --------------------------------------------------- | ----------------------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------- |
| [3207](https://www.rfc-editor.org/rfc/rfc3207.html) | SMTP Service Extension for Secure SMTP over TLS | Standards Track       | Obsoletes 2487; STARTTLS negotiation and state reset.                                          |
| [4954](https://www.rfc-editor.org/rfc/rfc4954.html) | SMTP Service Extension for Authentication       | Standards Track       | Obsoletes 2554; AUTH framework, not a permission model.                                        |
| [6409](https://www.rfc-editor.org/rfc/rfc6409.html) | Message Submission                              | Standards Track       | Obsoletes 4409; separates authenticated client submission from relay.                          |
| [8446](https://www.rfc-editor.org/rfc/rfc8446.html) | TLS 1.3                                         | Standards Track       | Current TLS protocol reference; implementation uses a maintained TLS library.                  |
| [8996](https://www.rfc-editor.org/rfc/rfc8996.html) | Deprecating TLS 1.0 and 1.1                     | Standards Track       | Updates 2246 and 4346; prohibits obsolete TLS versions for new secure deployments.             |
| [8314](https://www.rfc-editor.org/rfc/rfc8314.html) | Cleartext Considered Obsolete                   | Best Current Practice | Secure submission/access guidance; relevant to client-facing endpoints.                        |
| [7628](https://www.rfc-editor.org/rfc/rfc7628.html) | OAuth SASL Mechanisms                           | Standards Track       | OAuth bearer and initial-client-response SASL mechanisms; only if OAuth SMTP AUTH is selected. |

## DNS routing and domain authentication

| RFC                                                                                                                                                                                                                                                                     | Title                                | Category/status         | Relationship and use                                                                              |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ | ----------------------- | ------------------------------------------------------------------------------------------------- |
| [1034](https://www.rfc-editor.org/rfc/rfc1034.html) / [1035](https://www.rfc-editor.org/rfc/rfc1035.html)                                                                                                                                                               | Domain Names concepts/implementation | Standards Track         | Foundational DNS behavior; use current DNS libraries rather than reimplementing wire parsing.     |
| [2181](https://www.rfc-editor.org/rfc/rfc2181.html)                                                                                                                                                                                                                     | DNS Clarifications                   | Standards Track         | Clarifies DNS data and delegation semantics.                                                      |
| [2308](https://www.rfc-editor.org/rfc/rfc2308.html)                                                                                                                                                                                                                     | Negative Caching                     | Standards Track         | Negative answer caching and SOA-derived TTL behavior.                                             |
| [5890](https://www.rfc-editor.org/rfc/rfc5890.html), [5891](https://www.rfc-editor.org/rfc/rfc5891.html), [5892](https://www.rfc-editor.org/rfc/rfc5892.html), [5893](https://www.rfc-editor.org/rfc/rfc5893.html), [5894](https://www.rfc-editor.org/rfc/rfc5894.html) | IDNA2008                             | Standards Track         | Current internationalized-domain framework, protocol, validity, bidi, and rationale requirements. |
| [4033](https://www.rfc-editor.org/rfc/rfc4033.html), [4034](https://www.rfc-editor.org/rfc/rfc4034.html), [4035](https://www.rfc-editor.org/rfc/rfc4035.html)                                                                                                           | DNSSEC                               | Standards Track         | Authentication of DNS data; required only for a DANE profile.                                     |
| [7208](https://www.rfc-editor.org/rfc/rfc7208.html)                                                                                                                                                                                                                     | SPF                                  | Standards Track         | Obsoletes 4408; sender authorization evaluation and DNS lookup limits.                            |
| [6376](https://www.rfc-editor.org/rfc/rfc6376.html)                                                                                                                                                                                                                     | DKIM Signatures                      | Standards Track         | Signature generation/verification; updated by 8301 and 8463.                                      |
| [8301](https://www.rfc-editor.org/rfc/rfc8301.html)                                                                                                                                                                                                                     | DKIM Cryptographic Update            | Standards Track         | Updates 6376; algorithm/key-size requirements.                                                    |
| [8463](https://www.rfc-editor.org/rfc/rfc8463.html)                                                                                                                                                                                                                     | DKIM Ed25519-SHA256                  | Standards Track         | Updates 6376; optional algorithm profile.                                                         |
| [9989](https://www.rfc-editor.org/rfc/rfc9989.html)                                                                                                                                                                                                                     | DMARC                                | Standards Track         | Obsoletes 7489 and 9091; current base DMARC specification verified May 2026.                      |
| [9990](https://www.rfc-editor.org/rfc/rfc9990.html)                                                                                                                                                                                                                     | DMARC Aggregate Reporting            | Standards Track         | Obsoletes 7489; current aggregate-reporting family member.                                        |
| [9991](https://www.rfc-editor.org/rfc/rfc9991.html)                                                                                                                                                                                                                     | DMARC Failure Reporting              | Standards Track         | Obsoletes 7489; updates 6591; current failure-reporting family member.                            |
| [7489](https://www.rfc-editor.org/rfc/rfc7489.html)                                                                                                                                                                                                                     | Original DMARC specification         | Informational, obsolete | Historical interoperability reference; not the sole current normative source.                     |

## Transport policy and authentication results

| RFC                                                 | Title                  | Category/status | Relationship and use                                                                            |
| --------------------------------------------------- | ---------------------- | --------------- | ----------------------------------------------------------------------------------------------- |
| [7672](https://www.rfc-editor.org/rfc/rfc7672.html) | SMTP DANE TLS          | Standards Track | Authenticated TLS via DNSSEC/TLSA; optional future profile.                                     |
| [8461](https://www.rfc-editor.org/rfc/rfc8461.html) | MTA-STS                | Standards Track | HTTPS policy discovery/enforcement for SMTP TLS; future profile.                                |
| [8460](https://www.rfc-editor.org/rfc/rfc8460.html) | SMTP TLS Reporting     | Standards Track | Reporting for transport failures; future profile.                                               |
| [8689](https://www.rfc-editor.org/rfc/rfc8689.html) | REQUIRETLS             | Standards Track | Per-message TLS requirement signaling; optional and must not be advertised without enforcement. |
| [8601](https://www.rfc-editor.org/rfc/rfc8601.html) | Authentication-Results | Standards Track | Obsoletes 7601; trusted authentication-result header format.                                    |
| [8617](https://www.rfc-editor.org/rfc/rfc8617.html) | ARC                    | Experimental    | Optional forwarding-chain evaluation; not a launch dependency.                                  |

## Mailbox, privacy, and operations

| RFC                                                 | Title                     | Category/status | Relationship and use                                     |
| --------------------------------------------------- | ------------------------- | --------------- | -------------------------------------------------------- |
| [9051](https://www.rfc-editor.org/rfc/rfc9051.html) | IMAP4rev2                 | Standards Track | Hosted-mailbox future profile; obsoletes 3501.           |
| [1939](https://www.rfc-editor.org/rfc/rfc1939.html) | POP3                      | Standards Track | Optional mailbox access; no initial commitment.          |
| [3834](https://www.rfc-editor.org/rfc/rfc3834.html) | Automatic Responses       | Standards Track | Auto-response loop and null reverse-path guidance.       |
| [2505](https://www.rfc-editor.org/rfc/rfc2505.html) | Anti-Spam Recommendations | Informational   | Operational guidance, not a conformance standard.        |
| [8551](https://www.rfc-editor.org/rfc/rfc8551.html) | S/MIME 4.0                | Standards Track | Future end-to-end signing/encryption integration only.   |
| [9580](https://www.rfc-editor.org/rfc/rfc9580.html) | OpenPGP                   | Standards Track | Future message confidentiality/signing integration only. |

## Management API references

| RFC                                                 | Title                                                         | Category/status | Relationship and use                                                                                          |
| --------------------------------------------------- | ------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------- |
| [8259](https://www.rfc-editor.org/rfc/rfc8259.html) | The JavaScript Object Notation (JSON) Data Interchange Format | Standards Track | Current JSON syntax for the public API; parser limits and schema validation remain platform responsibilities. |
| [9110](https://www.rfc-editor.org/rfc/rfc9110.html) | HTTP Semantics                                                | Standards Track | Current HTTP semantics for API methods, status codes, caching, and conditional behavior.                      |

## Relationship cautions

- RFC 5321 transport does not define message-body semantics; RFC 5322 and MIME do.
- SMTP AUTH authenticates a client; authorization to send as a tenant/domain is platform policy.
- SPF authenticates a sending host for an identity; DKIM authenticates signed content; DMARC evaluates alignment and policy. None proves inbox placement.
- STARTTLS provides negotiated transport encryption but not universal end-to-end confidentiality or authenticated delivery unless a stronger policy such as DANE is applied.
- RFC 7489 remains useful for deployed legacy behavior, but current DMARC research uses RFC 9989/9990/9991.
- IANA’s [SMTP service-extension registry](https://www.iana.org/assignments/smtp/) is authoritative for registered EHLO keywords and references.
