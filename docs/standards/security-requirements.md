# Security requirements derived from standards

| ID          | Source                    | Threat                             | Required behavior                                                                                 | Owner          | Validation                      |
| ----------- | ------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------- | -------------- | ------------------------------- |
| SEC-STD-001 | RFC 5321 §§4.1, 4.5.3     | parser/resource exhaustion         | Bound lines, recipients, message bytes, connections, and command time                             | SMTP session   | fuzz/load/timeout tests         |
| SEC-STD-002 | RFC 5321 §7; RFC 2505     | open relay and abuse               | Require authenticated/authorized submission and verified sender policy                            | edge/policy    | unauthenticated relay negatives |
| SEC-STD-003 | RFC 3207 §4               | STARTTLS downgrade/state confusion | Reset state after TLS and never advertise unsupported capability                                  | TLS/session    | downgrade and reset tests       |
| SEC-STD-004 | RFC 4954 §§4–6            | credential theft/brute force       | Restrict mechanisms, require protected transport, rate-limit failures, rotate credentials         | identity       | auth abuse tests                |
| SEC-STD-005 | RFC 5322 §2.2; RFC 6532   | header injection/spoofing          | Reject control characters and validate folding/UTF-8; do not trust visible From for authorization | parser         | injection/Unicode tests         |
| SEC-STD-006 | RFC 7208 §§4–5            | DNS lookup exhaustion              | Enforce SPF lookup/void-query limits and classify temporary errors                                | auth resolver  | SPF limit tests                 |
| SEC-STD-007 | RFC 6376 §§3–5; RFC 8301  | DKIM forgery/weak crypto           | Enforce approved algorithms/key sizes and protect signing keys                                    | key/auth       | key and mutation tests          |
| SEC-STD-008 | RFC 8601                  | forged authentication result       | Add results only in a trusted boundary; strip/ignore untrusted claims                             | auth policy    | forged-header tests             |
| SEC-STD-009 | RFC 7672; RFC 8461        | false transport assurance          | Distinguish opportunistic TLS from authenticated/policy-required TLS                              | transport      | policy failure tests            |
| SEC-STD-010 | RFC 3461/3464/3834        | bounce amplification/backscatter   | Use null reverse paths for generated reports and suppress loops                                   | bounce         | report-loop tests               |
| SEC-STD-011 | RFC 2045/2046             | MIME parser exploitation           | Bound nesting/decoded output and quarantine malformed/unsafe content                              | parser/scanner | fuzz/resource tests             |
| SEC-STD-012 | Phase 01 tenant isolation | cross-tenant disclosure            | Carry tenant context through messages, events, logs, indexes, and backups                         | all services   | negative authorization tests    |

RFC compliance is necessary but insufficient: tenant isolation, quotas, abuse detection, secret management, key rotation, logging redaction, and incident response remain platform controls.
