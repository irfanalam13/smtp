# Standards scope and support profiles

Phase 01 selects transactional outbound delivery as the initial product. This scope prevents the standards inventory from treating every email protocol as a launch requirement.

| Profile                                     | Support level                                                                                        | Rationale                                                                                                                |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| A. SMTP submission and outbound delivery    | Required for initial release; security features required before production                           | Core product path: authenticated submission, durable acceptance, DNS routing, SMTP transport, TLS, status classification |
| B. Inbound SMTP processing                  | Planned future capability; current prototype is opt-in only                                          | Requires public MTA boundary, recipient policy, anti-abuse, inbound storage, and operational readiness                   |
| C. Hosted mailboxes                         | Out of scope for initial release; IMAP future, POP3 optional                                         | Mailbox state, synchronization, retention, search, quotas, and client compatibility are a separate product               |
| D. Email authentication and domain security | SPF/DKIM/DMARC required before production sending; MTA-STS/TLS-RPT planned; DANE/REQUIRETLS optional | Reputation and transport policy depend on domain authentication, but not every mechanism has equal deployment maturity   |
| E. Platform management/integration          | REST/OpenAPI and HTTPS required; OIDC/API-key standards required before production                   | Public management APIs and integrations are architectural boundaries, not SMTP protocol features                         |

## Initial-release profile

The initial delivery service should support bounded SMTP submission and outbound transport using RFC 5321 behavior, RFC 6409 submission semantics, RFC 5322/MIME message preservation, SIZE, STARTTLS, enhanced status codes, and safe DNS/MX routing. SMTP AUTH is required before exposing authenticated submission. DKIM signing and verified SPF/DMARC domain configuration are production launch blockers from Phase 01, even though their full implementations belong later.

SMTPUTF8 is **optional in the first interoperable release** and must not be advertised until end-to-end parsing, storage, DNS, DSN, and delivery behavior is implemented. Messages requiring SMTPUTF8 must be rejected or explicitly deferred when the destination does not advertise it; no lossy ASCII rewriting is permitted.

PIPELINING is optional until the client/server state machine has conformance coverage. DSN is planned before a mature customer delivery API. ARC is optional and not a launch dependency.

## Out-of-scope decisions

- **IMAP/POP3:** reconsider when mailbox provisioning, retention, synchronization, and user-client support are funded.
- **S/MIME/OpenPGP:** preserve encrypted content and avoid modification; reconsider only if the product owns user keys or offers signing/encryption.
- **DANE:** reconsider when DNSSEC validation, resolver operations, and customer policy requirements justify it.
- **MTA-STS and TLS reporting:** planned after outbound TLS telemetry and hosted policy retrieval are operational.
- **ARC:** optional interoperability feature; reconsider if forwarding ecosystems and a stable standards-track successor justify deployment.
- **Provider-specific policies:** monitored operational requirements, never presented as RFC obligations.
