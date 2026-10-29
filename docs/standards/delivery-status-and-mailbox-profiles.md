# Delivery status, abuse, privacy, and mailbox profiles

## DSN and bounce classification

RFC 3461 defines DSN negotiation, RFC 3463 defines enhanced status codes, RFC 3464 and RFC 6522 define report structures, and RFC 6533 extends the model for internationalized reports. The delivery service records the original SMTP class/code, enhanced status code where present, recipient scope, attempt number, and sanitized diagnostic.

Customer-facing status remains a projection:

| Protocol fact                                    | Customer status                         | Action                                                 |
| ------------------------------------------------ | --------------------------------------- | ------------------------------------------------------ |
| Remote 2xx after DATA                            | `delivered_to_next_hop`                 | No claim about inbox placement                         |
| Valid 4xx/temporary DNS/TLS/connection failure   | `deferred`                              | Retry with backoff and destination budget              |
| Valid 5xx/invalid recipient/Null MX              | `permanent_failure`                     | Suppression/bounce policy evaluation                   |
| Queue expiry or operator stop                    | `expired`/`administratively_terminated` | Audit reason; no further retry                         |
| Lost connection after possible remote acceptance | `ambiguous`                             | Retry may duplicate; never claim confirmed success     |
| Locally generated report                         | `bounce_processed`                      | Link to original message and preserve report semantics |

Null reverse paths are required for generated DSNs and automatic responses to avoid loops. RFC 3834 guidance is applied to suppress auto-response loops and backscatter. A customer-visible bounce must not be generated for an unauthenticated or forged report without a trusted association.

## Abuse and sender reputation

RFC 2505 is Informational guidance, not a universal conformance standard. Platform policy adds:

- no open relay;
- authenticated submission and verified sender domains;
- tenant/domain/recipient/destination quotas and fair scheduling;
- complaint, hard-bounce, spam-trap, and anomalous-volume signals;
- suppression and emergency suspension;
- stable egress identity, reverse DNS, SPF/DKIM/DMARC alignment, and provider-specific monitoring;
- customer consent/unsubscribe controls where bulk or marketing use is offered.

Mailbox-provider sender requirements are operational policies, not Internet standards. Their official sources and verification dates must be tracked separately before claiming interoperability.

## Privacy and message security

TLS does not equal end-to-end confidentiality. The initial platform preserves encrypted message content without interpreting it and excludes S/MIME (RFC 8551) and OpenPGP (RFC 9580) signing/encryption services. These are future integrations requiring key ownership, retention, metadata, legal, and client-compatibility decisions.

Logs and events exclude bodies, credentials, tokens, and unnecessary personal data. Restricted delivery metadata may retain bounded remote diagnostics for investigation. Raw content, backups, audit records, and derived indexes follow Phase 01 retention, encryption, tenant-isolation, deletion, and legal-hold requirements.

## Hosted mailbox profile

IMAP4rev2 (RFC 9051) is a future mailbox protocol. POP3 (RFC 1939) is optional and not part of the initial product. If mailbox work begins, the implementation must cover authentication/TLS, mailbox selection, UIDVALIDITY/UIDNEXT, flags, partial fetch, search, concurrent changes, quotas, and deletion semantics; it must not treat an SMTP queue as a mailbox store. RFC 8314 secure-access guidance applies to client access. A future mailbox ADR must decide IMAP-only versus IMAP+POP3.
