# Conformance and interoperability testing strategy

No protocol implementation or test harness is added in Phase 02. This plan defines later evidence.

## Test layers

1. **Normative unit tests** — each traceability requirement has deterministic parser, state, policy, and classification tests.
2. **Negative/security tests** — malformed commands, invalid UTF-8, injection, oversized input, open relay, unsafe DNS, forged auth results, credential abuse, and downgrade attempts.
3. **Integration tests** — SMTP session to durable acceptance, resolver to delivery scheduler, auth policy to tenant authorization, and event-to-bounce projections.
4. **Interoperability tests** — controlled conversations with known SMTP servers, TLS endpoints, DNS fixtures, and representative MIME clients. A passing peer test is evidence for that profile, not universal compliance.
5. **Fuzz/property tests** — command/reply framing, RFC 5322/MIME boundaries, DNS response parsing, enhanced status codes, and UTF-8 normalization.
6. **Load/chaos/recovery tests** — connection exhaustion, queue backlog, database outage, content-store outage, worker crash after remote `DATA`, resolver outage, and duplicate event delivery.

## Planned test matrix

### SMTP

Test valid/invalid command state, EHLO/HELO, multiline replies, envelope syntax, duplicate recipients, DATA termination, dot-stuffing, line endings, SIZE, 8BITMIME, STARTTLS reset, AUTH failures, PIPELINING ordering, timeouts, graceful shutdown, and SMTPUTF8 negotiation.

### Message/MIME

Test folded headers, address grammar, Message-ID/Date handling, multipart nesting and boundaries, base64/quoted-printable, encoded words, RFC 2231 parameters, invalid MIME, header injection, Unicode, size accounting, raw-byte preservation, and fuzz cases.

### DNS/authentication

Use fixtures for MX preference, equal preference, absent MX fallback, Null MX, CNAME/address behavior, NXDOMAIN, SERVFAIL, timeout, TTL/negative caching, IDNA, SPF result classes and lookup limits, DKIM key rotation/canonicalization, DMARC alignment/policy/reporting, and DNSSEC/TLSA where enabled.

### TLS

Test valid/expired/wrong-host certificates, TLS version rejection, STARTTLS success/failure/reset, opportunistic versus required policy, MTA-STS modes, TLS reports, DANE validation, REQUIRETLS propagation, certificate renewal, and remote interoperability.

## Tooling policy

Use maintained protocol libraries and test peers only after verifying their current documentation, license, and limitations. A public SMTP test service cannot prove every RFC requirement and must not receive production messages or secrets. Fuzzing and chaos runs use synthetic domains/content. Tests must capture the RFC/section and requirement ID they exercise.

## Acceptance evidence

An implementation phase may mark a requirement `validated` only when the test result, fixture/version, environment, and failure behavior are recorded. No single RFC test suite proves complete conformance. Regression tests are required when a standards update, dependency update, provider interoperability issue, or security advisory changes behavior.
