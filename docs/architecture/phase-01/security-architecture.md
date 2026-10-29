# Security architecture

## Trust boundaries

```text
Internet SMTP/API clients → public edge → protocol/control services
Remote MX servers ↔ outbound transport
Control services → tenant data stores
Data services → queue/content stores
Platform operators → audited administrative boundary
Event consumers → least-privilege event subscriptions
```

Untrusted input includes SMTP commands, message bytes, DNS answers, API JSON, webhook responses, and remote SMTP replies. No remote mail server, customer tenant, or event consumer is trusted merely because it is reachable.

## Identity and authorization

- API users use OIDC-issued identity; service accounts use scoped, rotatable API keys or short-lived workload credentials.
- API keys are generated with a CSPRNG, stored as a verifier plus safe prefix, shown once, scoped, expiring, revocable, and never placed in URLs or logs. This preserves the accepted repository decision.
- SMTP AUTH credentials are separate from API keys and must support rotation, brute-force limits, and emergency revocation.
- The authenticated principal determines tenant context. Every resource operation checks tenant ownership, role, scope, and object state.
- Internal calls use workload identity/mTLS or signed service tokens with audience, expiry, and least privilege. Retries preserve idempotency keys.
- Operators use separate privileged roles, MFA through the identity provider, just-in-time access where available, and immutable audit records.

Control-plane outages: data-plane processes use a signed policy snapshot with a bounded lease. New tenants/identities and policy changes stop when the lease expires; emergency suspension is distributed as a high-priority control event and polled independently. Already accepted messages remain governed by the last valid policy and abuse kill switch.

## Email-specific controls

- No open relay: unauthenticated submission is rejected; server-to-server inbound is limited to verified domains/mailbox routes and network policy.
- Bound commands, lines, recipients, message size, connections, parser depth, and timeouts.
- Normalize/validate headers and reject injection/control characters.
- Resolve MX and validate addresses before connecting; block loopback, private, link-local, multicast, unspecified, documentation, and reserved ranges except explicit fixture mode.
- Validate STARTTLS certificates for the MX hostname; do not silently downgrade after an advertised required handshake fails.
- SPF, DKIM, and DMARC are standards-integrated checks/signing responsibilities; advisory DNS diagnostics do not authorize a sender. DKIM key storage/rotation is a P1 prerequisite for production sending.
- Rate-limit authentication, submission, recipients, domains, and destination concurrency. Detect complaints, spam traps, bounce amplification, credential compromise, and anomalous volume.
- Malware/phishing scanning is an integration point with fail-closed or quarantine policy selected per product risk; it is not claimed as implemented by the current repository.

## Tenant isolation and cryptography

Tenant IDs are mandatory on metadata, queue records, events, audit entries, and content references. Database constraints, service roles, application authorization, and negative cross-tenant tests provide defense in depth. Search indexes, backups, logs, and analytics use tenant filters and least privilege.

TLS protects external and internal connections. Data at rest uses provider/database encryption plus envelope keys for raw message content where the threat model requires it. KMS/HSM keys are separate from ordinary configuration secrets, rotated on a schedule, access-audited, and revocable. Backups are encrypted and tested for restore; deletion requires content, metadata, projections, events, and backup-retention treatment.

## STRIDE threat model

| Threat                 | Asset/path                              | Likelihood/impact | Mitigation                                                      | Residual risk and validation                           |
| ---------------------- | --------------------------------------- | ----------------- | --------------------------------------------------------------- | ------------------------------------------------------ |
| Spoofing               | API key, SMTP AUTH, operator identity   | High/High         | OIDC, scoped keys, MFA, rotation, rate limits                   | Credential theft; auth abuse tests and incident drills |
| Tampering              | Message, queue state, DNS answer, event | Medium/High       | TLS, lease tokens, signed config, bounded resolver, constraints | Compromised operator; audit and integrity tests        |
| Repudiation            | Delivery and admin actions              | Medium/Medium     | Correlation IDs, immutable audit, signed events                 | Clock/source issues; audit review                      |
| Information disclosure | Raw body, tenant metadata, logs/backups | Medium/High       | Encryption, tenant checks, redaction, retention                 | Misconfigured consumer; access tests and DLP review    |
| Denial of service      | SMTP/API/queue/destination resources    | High/High         | Limits, fair scheduling, bulkheads, circuit breakers, quotas    | Distributed abuse; load and chaos testing              |
| Privilege escalation   | Tenant/admin/service roles              | Medium/High       | RBAC, audience checks, separate roles, no tenant from request   | Policy drift; authorization matrix tests               |

Detailed cryptographic algorithms, standards compliance, and credential implementations belong to Phases 02 and 07.
