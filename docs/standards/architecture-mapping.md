# Standards-to-architecture mapping

| Standards area          | Phase 01 responsibility                       | Main references                                        | Dependencies/tests                         | Target phase |
| ----------------------- | --------------------------------------------- | ------------------------------------------------------ | ------------------------------------------ | ------------ |
| SMTP submission         | SMTP submission component                     | RFC 5321, 6409, 4954, 3207, 1870                       | identity, TLS, state/negative tests        | 02/07        |
| SMTP transport          | queue/delivery worker                         | RFC 5321, 2920, 6152, 3463                             | resolver, TLS, retry/ambiguity tests       | 02/10        |
| Message/MIME            | parser and content store                      | RFC 5322, 2045, 2046, 2047, 2231                       | bounded parsing, raw preservation/fuzz     | 02/06        |
| DNS routing             | resolver/routing module                       | RFC 5321 §5, 1034/1035, 2181, 2308, 7505               | resolver safety, MX/Null MX tests          | 02           |
| SPF/DKIM/DMARC          | domain/authentication policy                  | RFC 7208, 6376, 8301, 8463, 9989–9991                  | DNS, KMS, alignment/report tests           | 07/08        |
| TLS                     | transport security and certificate management | RFC 3207, 8446, 8996, 7672, 8461, 8460, 8689           | cert/policy/telemetry tests                | 07/08/09     |
| SMTP AUTH               | identity and submission authorization         | RFC 4954, 7628, SASL registry                          | OIDC/API key separation, brute-force tests | 07           |
| DSN/status              | delivery state and bounce processor           | RFC 3461, 3463, 3464, 6522, 6533, 3834                 | event/idempotency/null reverse path        | 08           |
| Internationalized email | session, parser, resolver, bounce             | RFC 6530–6533, current IDNA specs                      | Unicode/IDNA capability tests              | 02/08        |
| Mailbox protocols       | future mailbox service                        | RFC 9051, 1939, 8314                                   | mailbox store/auth/UID tests               | future       |
| API/integrations        | API gateway/control plane/webhooks            | HTTP/JSON/OAuth/OIDC/OpenAPI; no universal webhook RFC | contract/security/idempotency tests        | 08           |
| Audit/security          | audit and observability                       | RFC 8601 plus Phase 01 controls                        | trusted headers, redaction, trace tests    | 07/09        |

## Architecture discrepancy review

Phase 01 assigned protocol work to suitable Rust components and authentication/policy to the control plane. No boundary needs a redesign. Two corrections are required before production implementation:

1. The Phase 01 statement that DKIM is a production prerequisite is reinforced by the standards profile; DNS diagnostics alone are not sender authentication.
2. The current outbound implementation's opportunistic/required STARTTLS behavior must be expanded with the selected certificate, TLS version, MTA-STS/DANE/REQUIRETLS policy decisions before claiming a production transport profile.
