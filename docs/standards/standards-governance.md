# Standards governance, change, and deprecation

## Ownership

The architecture owner maintains the inventory and relationship map. Protocol, security, and platform owners review their rows and traceability requirements. Product/legal owners review retention, provider requirements, privacy, and licensing implications.

## Review triggers

- New RFC or RFC status change in SMTP, DMARC, TLS, DNSSEC, MIME, SASL, IMAP, or security work groups.
- RFC update/obsoletes relationship or published erratum affecting a selected section.
- IANA SMTP, DNS, message-header, media-type, or service-port registry change.
- TLS/library/security advisory or cryptographic deprecation.
- Interoperability regression or mailbox-provider sender-policy change.
- New product profile such as inbound mail, mailbox access, or internationalized sending.

For each trigger, record source, impact, decision, updated requirement IDs, test changes, rollout/compatibility plan, and ADR update if the architecture changes.

## Support policy

- Do not advertise an SMTP extension until all required semantics and failure paths are implemented.
- Accept legacy behavior only when it does not create an open relay, credential, parser, or downgrade risk.
- Reject obsolete TLS and weak cryptography even if a legacy client expects them; document migration guidance.
- Preserve RFC 5321/5322 interoperability where safe, but do not perform lossy address or header rewriting.
- Mark an implementation `validated` only after behavior tests and interoperability evidence.

## Deprecation policy

Every excluded/deprecated feature records reason, security impact, interoperability impact, replacement, customer communication, test coverage, and review trigger. RFC 7489 is retained for legacy DMARC compatibility but RFC 9989/9990/9991 are the current research baseline. ARC remains Experimental and is not a required trust signal. POP3, S/MIME, OpenPGP, DANE, MTA-STS, and REQUIRETLS remain separately gated capabilities rather than silently implied support.

## Provider-specific requirements

Mailbox-provider requirements are tracked in a separate operational register with provider, official source, verification date, affected profile, change monitor, and customer-impact plan. They must never be copied into the RFC compliance matrix as Internet-standard obligations.
