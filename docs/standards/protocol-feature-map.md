# Protocol feature map

| Feature            | Standards                       | Owner                     | Dependencies                   | Priority             | Acceptance criteria                                 |
| ------------------ | ------------------------------- | ------------------------- | ------------------------------ | -------------------- | --------------------------------------------------- |
| SMTP session       | RFC 5321                        | SMTP submission/transport | bounded parser, reply model    | P0                   | valid/invalid state matrix passes                   |
| Message submission | RFC 6409, RFC 4954, RFC 3207    | submission + identity     | TLS, tenant authorization      | P0                   | authenticated sender cannot cross tenant/domain     |
| Message framing    | RFC 5321, RFC 5322              | submission/parser         | CRLF, dot transparency         | P0                   | stored bytes round-trip correctly                   |
| MIME               | RFC 2045/2046/2047/2231         | message parser            | limits, charset policy         | P0                   | multipart/encoding negative tests pass              |
| SIZE/8BITMIME      | RFC 1870/6152                   | SMTP session              | content accounting             | P0/conditional       | advertised limits are true                          |
| PIPELINING         | RFC 2920                        | transport                 | response correlation           | P1                   | no desynchronization under failures                 |
| DNS routing        | RFC 5321 §5, RFC 7505, DNS RFCs | resolver/transport        | safe address filtering         | P0                   | MX, Null MX, DNS faults behave correctly            |
| TLS                | RFC 3207, RFC 8446, RFC 8996    | TLS/transport             | certificates, policy           | P0                   | no obsolete TLS; required failure is not downgraded |
| SMTP AUTH          | RFC 4954, SASL registry         | identity/submission       | credential lifecycle           | P0 before public     | selected mechanisms and authorization tested        |
| SPF                | RFC 7208                        | domain/auth policy        | resolver limits                | P0 before production | all result classes and limits tested                |
| DKIM               | RFC 6376/8301/8463              | signing/auth policy       | key manager, canonicalization  | P0 before production | signature survives intended serialization           |
| DMARC              | RFC 9989/9990/9991              | auth policy/reporting     | SPF/DKIM, alignment            | P0 before production | policy/alignment/reporting tests                    |
| DSN/bounce         | RFC 3461/3463/3464/6522/6533    | delivery/bounce           | event model, null reverse path | P1                   | customer status preserves protocol facts            |
| SMTPUTF8           | RFC 6531/6532/6533              | parser/session/bounce     | Unicode/IDNA                   | P1                   | no lossy fallback                                   |
| MTA-STS/TLS-RPT    | RFC 8461/8460                   | transport/observability   | HTTPS policy, telemetry        | P1                   | policy modes and reports are accurate               |
| DANE               | RFC 7672 + DNSSEC               | transport/resolver        | validated DNSSEC               | optional             | TLSA failure semantics tested                       |
| Auth results       | RFC 8601                        | auth policy               | trusted boundary               | P1                   | client headers cannot forge platform result         |
| ARC                | RFC 8617                        | auth policy               | forwarding model               | optional             | never used as sole acceptance authority             |
| IMAP/POP3          | RFC 9051/1939/8314              | mailbox service           | mailbox store, auth            | P2                   | separate mailbox conformance plan                   |
