# GitHub Copilot Agent Prompt — SMTP Platform Gap Remediation and Production Readiness

**Repository:** `https://github.com/irfanalam13/smtp`
**Target branch:** `main` (create a dedicated feature branch before changes)
**Audit snapshot:** 2026-11-01
**Goal:** Turn the existing repository into a correct, secure, operable email platform by completing its real gaps. Do not rebuild working subsystems or claim production readiness without evidence.

---

## ROLE AND OPERATING MODE

Act as a principal mail-systems engineer, SMTP/ESMTP protocol implementer, Rust and Go security engineer, PostgreSQL data-platform engineer, and production SRE. Work inside this repository. Inspect the actual code, migrations, tests, configuration, workflows, and documentation before editing.

This is an implementation task, not a high-level architecture proposal. Make working, tested changes. Do not stop after writing a plan, checklist, interfaces, placeholders, TODOs, or mock implementations. Do not mark a feature complete unless its runtime wiring, persistence, failure handling, tests, configuration, and documentation are complete.

### Non-negotiable repository constraints

1. Preserve the current technology and service boundaries unless a verified defect makes a small, justified change necessary:
   - Rust/Tokio SMTP ingress, queue, delivery, shared message types, IMAP and mailbox services.
   - Go control plane and `smtp-auth` identity package.
   - Next.js dashboard.
   - PostgreSQL migrations and durable state.
2. Reuse and extend the existing modules. Do not create a second SMTP server, duplicate queue, competing auth system, parallel message model, or redundant mailbox store.
3. Keep SMTP envelope sender/recipient data separate from RFC 5322 message headers. Preserve raw-message integrity and DKIM semantics deliberately.
4. Keep tenant isolation at every API, queue, storage, delivery, credential, mailbox, webhook, DNS-key and reporting boundary.
5. Do not expose an open relay. Never allow unauthenticated clients to relay mail to arbitrary external recipients.
6. Do not log message bodies, attachments, passwords, API keys, OAuth codes, session tokens, reset tokens, verification tokens, DKIM private keys, or raw sensitive headers.
7. Use additive, reversible PostgreSQL migrations. Never silently drop customer data or make a destructive migration the default.
8. Maintain fail-closed security behavior for public ingress, authentication, signing-key availability, and destination safety. Return correct temporary SMTP failures where retry is safe; do not silently downgrade TLS.
9. Do not introduce Kafka, Redis, Kubernetes, a vector database, a cloud provider, or another large dependency merely because it is fashionable. First prove that the current PostgreSQL-backed design cannot meet a measured requirement. Provider-specific deployment work must remain configurable and documented where no provider has been selected.
10. Do not build unrelated AI/global infrastructure phases as a substitute for finishing basic mail delivery, security, storage, and operations.
11. Do not advertise SMTP/IMAP/POP3 capabilities that are not fully implemented end-to-end. Capability strings must reflect the actual listener profile and runtime configuration.
12. Keep test-only and development behavior impossible to enable accidentally in production. Development OAuth simulation, development identity adapters, local SMTP relays, and loopback exceptions must be strictly environment-gated.

---

# PART A — FIRST AUDIT THE CURRENT REPOSITORY

Before coding, inspect the entire active implementation and produce:

`docs/audits/smtp-production-gap-audit.md`

The audit must identify, for every requirement below:

- Status: `IMPLEMENTED`, `PARTIAL`, `MISSING`, `BROKEN`, `DOCUMENTATION-ONLY`, or `OUT-OF-SCOPE`.
- Actual source files, public APIs, database tables/migrations and executable entry points.
- Existing tests and the exact command that runs them.
- Missing runtime wiring, operational dependency, failure case, security boundary or interoperability evidence.
- The minimum safe implementation plan and prerequisite dependencies.

Do not infer status from a README, phase plan, filename, interface, struct, or isolated unit test. Follow the path from the listener/API request through validation, authorization, persistence, queue, worker, external protocol and persisted final result.

## Known repository facts to validate, not blindly assume

The repository already has substantial code. Confirm its current state before changing it:

- `smtp-session` (`smtp-ingress`) implements a bounded SMTP submission/session profile with STARTTLS, optional implicit TLS, TLS-gated AUTH PLAIN, tenant-scoped PostgreSQL credentials, sender authorization, rate limiting, connection bounds and graceful drain.
- `smtp-queue` has PostgreSQL persistence, idempotent enqueue, lease-based claims, retry state and recipient-level delivery records.
- `smtp-delivery` has a real outbound SMTP worker, MX-based routing, destination-address filtering, bounded replies/timeouts, opportunistic/required STARTTLS behavior, recipient outcomes and an Ed25519 DKIM signer. Its configuration also contains authenticated relay and local Mailpit modes; inspect their actual use before describing either as absent.
- Inbound processing has recipient routing, same-tenant fanout, PostgreSQL persistence, ClamAV and SpamAssassin adapters, HMAC-keyed shared PostgreSQL rate limits, and an email authentication adapter for SPF/DKIM/DMARC.
- The Go API already has password signup/login/recovery routes, browser sessions, JWT utility/configuration, generic OIDC, Google OAuth and GitHub OAuth code. These must be audited and hardened, not replaced with a second authentication stack.
- `smtp-imap` is an actual initial implicit-TLS IMAP4rev1 listener backed by the mailbox store. It is a bounded profile, not complete IMAP4rev1/IMAP4rev2 interoperability.
- `smtp-pop3` is currently a parser/simulated session, not a working POP3 listener or real mailbox service.
- Mailbox message bytes remain stored inline in PostgreSQL. `mailbox-service/src/retention.rs` currently cleans expired append-idempotency records; that is not a complete message-retention or object-storage lifecycle.
- CI already runs important Rust, Go, dashboard, PostgreSQL, and security checks. Extend it with missing protocol/integration evidence instead of replacing it.

The documents have stale or contradictory descriptions. For example, some documentation still describes the inbound projection consumer or IMAP listener as unimplemented even though newer code/status describes or implements them. Reconcile the docs with the actual executable code and tests.

---

# PART B — P0 SECURITY AND MESSAGE-CORRECTNESS FIXES

Complete this section first. Add regression tests for every issue.

## B1. Fix account-verification token disclosure

Inspect `smtp-api/internal/controlplane/password_auth_handlers.go`, especially `signUp`.

Current verified concern: signup constructs a verification URL containing a one-time secret and includes `verification_url` in the HTTP response in production as well as development. It also ignores the return value from `sendAccountEmail`.

Implement all of the following:

- Never return a secret-bearing verification URL or raw verification token from a production signup endpoint.
- Production response must be generic and non-enumerating, such as “If registration can proceed, a verification email will be sent.” Do not disclose whether an email already exists.
- Persist only a cryptographic hash of the high-entropy verification token. Preserve single-use semantics, purpose binding, expiry, constant-time comparisons where applicable, and transaction boundaries.
- Make account email delivery durable and retryable. Prefer the repository's existing PostgreSQL outbox/event patterns. Commit account state and the email-delivery intent atomically; workers may send after commit.
- Do not ignore notification failures. A failed provider call must leave a retryable event with bounded exponential backoff, attempt metadata, idempotency and a terminal/dead-letter state with operational visibility.
- Do not log the URL, token, recipient's sensitive details or message content. Redact credentials in errors and traces.
- Development-only test helpers must require an explicit development environment and must not be enabled by `!SecureCookies` alone in a production configuration.
- Apply the same review to password reset, email resend, change-password and session-revocation flows. Ensure tokens are single-use, purpose-specific, expiring, hashed at rest, rate-limited and invalidated after use.

Acceptance tests:
- Production signup JSON never contains `verification_url`, the raw token or a token-bearing link.
- Unknown and existing accounts have indistinguishable public responses where appropriate.
- Email provider outage leaves a durable retryable event and does not silently lose the notification.
- Verification/reset token cannot be replayed, used for another purpose or used after expiry.
- Logs and returned errors contain no raw secret.

## B2. Replace process-local production auth throttling

Audit all calls to `h.Store.Allow` and every rate-limit implementation in Go and Rust.

- Process-local limits may be used only for explicitly isolated development/test behavior.
- Implement shared, persistent production limits for signup, login, password recovery, verification resend, OAuth initiation/callback abuse, API-key failures, session issuance, SMTP AUTH and other sensitive endpoints.
- Use PostgreSQL atomic updates or the existing shared limiter abstractions. Keep the counters bounded, expiry/pruning bounded and keys pseudonymized with a deployment-injected HMAC key where identifiers/IPs are involved.
- Define multiple dimensions (IP, account/email hash, tenant and operation) without allowing an attacker-controlled key explosion.
- Ensure proxy-derived client IP is trusted only from explicitly configured reverse proxies. Never trust arbitrary `X-Forwarded-For` supplied by the client.
- Fail closed or use a deliberately documented safe temporary-unavailable policy if shared abuse controls cannot be reached. Do not silently fall back to unbounded process-local enforcement in production.

Acceptance tests must run with two independent API instances sharing one disposable PostgreSQL database and demonstrate a single effective limit.

## B3. Audit Google, GitHub and generic OIDC login as security protocols

Keep existing provider support and inspect the provider-specific flows rather than assuming they are interchangeable.

- Validate OAuth `state` with constant-time comparison and bind it to a short-lived, single-use server-side flow record or properly protected cookie.
- Use PKCE where supported by the provider; do not store or generate a PKCE verifier that is not actually enforced during code exchange.
- Validate OIDC `nonce` against a verified ID token for OIDC providers. Do not treat ordinary OAuth userinfo as proof of an ID token nonce.
- Validate exact redirect URI, issuer, audience, expiry, signature, subject and provider-specific verified-email semantics. Never link a password account solely on an unverified email address.
- Fetch provider responses with bounded timeouts, response-body limits, expected content types, status checks and safe error handling. Do not follow arbitrary redirects or permit SSRF through configurable provider endpoints without a strict allowlist.
- Make callback flow state single-use and expire it. Prevent replay, login CSRF and account-linking confusion.
- Ensure OAuth flow cookies, browser-session cookies and CSRF cookies have correct `Secure`, `HttpOnly`, `SameSite`, `Path`, host-only and expiry attributes for their roles. Use `HttpOnly` for secrets; CSRF protections must remain usable by the client.
- Separate login, signup/provisioning and linking-an-existing-account flows. Account linking requires proof of control of both identities or a recent authenticated session.
- Apply shared rate limits to starts, callbacks and provider errors.
- Add tests for bad state, replayed state, missing/invalid PKCE, nonce mismatch, unverified email, wrong issuer/audience, expired ID token, callback errors, redirect mismatch and provider outage.

Do not create new Google/GitHub/OIDC handlers if an existing route can be safely repaired.

## B4. JWT and browser-session lifecycle hardening

The repository already has `JWTConfig` issue/verify behavior and persisted opaque browser sessions. Retain these designs where sound.

- Review JWT algorithm pinning, issuer/audience/subject/tenant/scope/role validation, expiration, not-before/issued-at handling, maximum token size, duplicate/unknown claims, and key rotation.
- Add a versioned key identifier (`kid`) and a safe key-rotation strategy if bearer JWTs are exposed to clients. Do not accept arbitrary algorithms or weak/default production secrets.
- Ensure signing keys are deployment secrets, never committed or defaulted in production. Fail startup for weak or absent production secrets.
- Define explicit API access-token expiry and refresh-token/session rotation; revoke refresh/session material on password change, recovery, account disablement, logout-all, tenant membership removal and security events.
- Preserve opaque hashed browser-session tokens and CSRF protection. Check cookie/session rotation, idle and absolute expiry, concurrent refresh replay, device revocation and audit records.
- Add DB integration and concurrency tests. Do not claim MFA or passkeys are implemented unless real WebAuthn/TOTP verification, enrollment, recovery, revocation, step-up policy and tests exist.

MFA/passkeys are a high-priority production identity requirement for privileged account/operator actions. Implement them using maintained libraries and WebAuthn/TOTP standards only after the identity model and recovery threat model are documented. Do not invent cryptography.

## B5. Add required SMTP trace header handling

Inspect `smtp-session/src/inbound/mod.rs` and `docs/architecture/inbound-mail.md`.

The current design states that accepted raw bytes are preserved unchanged and that peer/listener/TLS connection metadata is stored separately without prepending a `Received` header. For a receiving SMTP server, RFC 5321 §4.4 requires the receiving server to prepend trace information (`Received`) to the message. Implement this correctly.

- Generate a trusted `Received` trace field from the server's own observed peer address, receiving hostname/listener, transport/TLS context, and timestamp.
- Never trust a client-supplied `Received` header as the current hop's trace. Do not overwrite or reorder existing trace headers.
- Preserve the original accepted wire bytes or a cryptographic digest for forensic/idempotency purposes if needed, but store and deliver the correctly traced message representation. Define exactly which byte representation is signed, hashed, indexed, retained and delivered.
- Ensure trace-header generation prevents CRLF/header injection and never includes untrusted raw HELO values without strict safe validation/escaping.
- Ensure idempotent retry detection remains consistent when the trusted trace is generated. A retry of the same SMTP transaction must not create a duplicate mailbox message or a different externally delivered message because a fresh timestamp was added before the idempotency decision.
- Do not break valid DKIM signatures by rewriting signed headers/body. Prepending a new trace header should preserve the original message octets and header order below it; add RFC test fixtures for valid pre-existing signatures.
- Cover multiple receiving hops, malformed source headers, IPv4/IPv6, TLS and non-TLS connections, forged Received values, retries and trace formatting.

Document the trace and raw-byte model explicitly.

---

# PART C — MAKE SMTP LISTENER ROLES EXPLICIT AND SAFE

The product must distinguish message submission from Internet mail relay/receipt. SMTP submission is a separate service role from SMTP relay; follow RFC 6409 and the relevant SMTP specifications.

## C1. Define listener profiles

Introduce an explicit, validated runtime profile for each public listener instead of relying on one ambiguous SMTP configuration to represent every use case.

### Port 25 — inbound MTA / relay reception

- Accept inbound messages only for active local recipients on active, verified domains, subject to inbound policy.
- Do not require user AUTH for ordinary external-to-local receiving.
- Never relay unauthenticated external-to-external traffic.
- Reject unknown recipients and unverified/non-local domains safely without leaking tenant existence.
- Apply per-IP, per-domain, per-tenant, recipient and concurrency controls.
- Perform SPF/DKIM/DMARC policy evaluation and malware/spam scanning according to configured inbound policy.
- Return `250` only after the acceptance transaction is durably committed and recoverable.

### Port 587 — authenticated message submission

- Require STARTTLS before AUTH.
- Require authenticated tenant/user/API credential as appropriate.
- Enforce sender identity/domain authorization and tenant limits.
- Do not advertise AUTH before TLS is established.
- Keep submission authorization separate from public inbound MTA recipient routing.

### Port 465 — implicit-TLS submission

- TLS handshake occurs before the SMTP greeting.
- Enforce the same submission authentication, sender authorization and tenant limits as port 587.

If the existing server can serve multiple listener profiles safely, reuse it with explicit per-listener configuration. Otherwise introduce the smallest service refactor that creates independent listener configs while sharing the common protocol/session implementation. Do not duplicate the SMTP parser or state machine.

Make bind addresses, ports, TLS policy, AUTH policy, inbound enablement, domain routing and advertised extensions explicit in config. Validate impossible or dangerous combinations on startup. Local development must bind loopback unless explicitly configured otherwise.

## C2. Complete SMTP core semantics before adding extensions

Audit RFC 5321 command and state handling, parser framing, reply text/codes, timeouts, command ordering, `MAIL FROM`, `RCPT TO`, `DATA`, transaction reset and graceful shutdown.

- Verify HELO/EHLO, MAIL, RCPT, DATA, RSET, NOOP, QUIT, HELP and VRFY behavior. Unsupported VRFY/EXPN must not reveal users.
- Enforce recipient limits, line length, command length, message size and deadlines consistently.
- Validate null reverse-path (`<>`) for generated bounce/notification messages without allowing spoofed use for normal submission.
- Enforce local-recipient-only behavior on inbound listener and tenant sender authorization on submission listener.
- Distinguish permanent recipient rejection, transient dependency failure and post-acceptance uncertain outcomes.
- Treat database commit as the durable acceptance boundary. A positive completion response after DATA must never be sent before durable commit.
- After a durable `250`, ensure all subsequent failures are recovered through durable queue/event state and standards-compliant notification rather than silently dropping the message.
- Preserve SMTP transaction reset semantics after RSET, DATA completion, errors and STARTTLS.
- Test slowloris clients, connection exhaustion, disconnects mid-command/DATA, Postgres failure before/after transaction commit, cancellation, TLS handshake failure and graceful shutdown.

## C3. ESMTP extensions: implement only end-to-end

Existing profile correctly leaves some extensions unadvertised. Keep that behavior until all dependencies and tests exist.

Prioritize in this order:

1. **PIPELINING (RFC 2920):** implement command pipelining with a response queue that preserves ordering, bounded in-flight commands and transaction semantics. Never process an unsafe sequence concurrently or desynchronize replies. Test valid/invalid pipelined transactions and backpressure.
2. **DSN (RFC 3461, 3463, 3464 and applicable updates):** support `ENVID`, `RET`, `NOTIFY`, `ORCPT`, correct recipient delivery state, null reverse path for DSNs, and standards-compatible DSN generation. Do not advertise DSN before reports persist and pass interoperability tests.
3. **SMTPUTF8/IDNA (RFC 6531/6532/6533):** implement only after mailbox local-part Unicode, IDNA domain canonicalization, normalization, database uniqueness/collation, DNS/MX resolver, queue envelope, outbound SMTP, IMAP and API all preserve the same canonical identity. Never lowercase or normalize the case-sensitive local-part incorrectly. Reject invalid or unsupported Unicode safely.
4. **CHUNKING/BDAT and BINARYMIME (RFC 3030):** implement bounded streaming, chunk accounting, cancellation and binary-safe storage first. Do not buffer unbounded messages in memory.
5. **REQUIRETLS and other extensions:** implement only with explicit end-to-end policy and destination behavior.

EHLO capability advertisement must be computed from the actual listener profile, TLS state, tenant config, implemented protocol handlers and runtime prerequisites. Add a test asserting that each advertised extension has a complete passing acceptance suite.

---

# PART D — COMPLETE OUTBOUND MTA DELIVERY (CRITICAL)

The queue and real worker already exist. Complete the production gap in `smtp-delivery` and `smtp-queue` without replacing their durable per-recipient design.

## D1. Delivery control and resilience

- Add shared per-tenant, per-sender-domain, per-destination-domain, per-recipient and destination-IP rate limits with explicit configurable budgets.
- Add per-domain circuit breakers, bounded concurrency, queue backpressure, retry-after handling, and fairness so one tenant/domain cannot starve others.
- Use bounded DNS caching that respects TTL and negative caching semantics. Record safe resolver diagnostics and preserve DNS error classes (NXDOMAIN, SERVFAIL, timeout, Null MX, no MX/implicit MX).
- Harden destination address filtering against the complete current IANA special-purpose ranges, private/reserved/multicast/link-local addresses, IPv4-mapped IPv6, rebinding, mixed unsafe answer sets and redirects. Validate resolved IPs and connect to the validated IP without a second hostname lookup.
- Re-audit candidate MX fallback and IP fallback order. Do not retry a message after an ambiguous post-DATA outcome unless the protocol state proves the remote server did not accept it.
- Maintain lease renewal/fencing. A stale worker must never overwrite a newer worker's result. Add tests for lease loss during DNS, connect, TLS, DATA, final reply and DB result persistence.
- Make graceful worker shutdown stop new claims and finish or safely mark current outcomes. Persist `uncertain` state if remote acceptance cannot be known.
- Bound message size, reply line length, multiline SMTP reply bytes, DNS candidates, socket count, response deadlines and total attempt duration.
- Persist safe per-recipient attempt history, SMTP reply code, enhanced status code, remote MX, TLS mode/certificate result, attempt timestamps and categorized error. Do not store credentials or raw message content in attempt logs.

## D2. Tenant/domain TLS policy and recipient-domain policy

- Support a clear policy model: opportunistic STARTTLS, required TLS, and recipient-domain MTA-STS policy where configured.
- If STARTTLS is advertised but negotiation/certificate validation fails, do not silently downgrade.
- Implement MTA-STS according to RFC 8461: policy discovery, HTTPS retrieval/validation, DNS TXT discovery, policy modes (`none`, `testing`, `enforce`), `mx` patterns, bounded cache/expiry and safe failure behavior. Keep a policy result/audit trail per destination domain.
- Implement TLS-RPT according to RFC 8460: aggregate privacy-safe TLS negotiation failure reports, validate/report destinations and send reports through the durable outbox. Do not leak message bodies or recipient content.
- Treat DANE SMTP (RFC 7672) as a separately gated capability requiring DNSSEC validation and correct TLSA semantics. Do not claim DANE from ordinary DNSSEC or TLS support alone.
- Support authenticated smarthost relay configuration already present in the repository. Verify SMTP AUTH mechanisms, implicit TLS/STARTTLS behavior, sender restrictions, secret redaction, tenant scoping and relay failover. Keep local Mailpit restricted to explicit development mode.
- Ensure delivery worker selects direct MX, relay or Mailpit queues without mode confusion.

## D3. Sender identity and outbound DKIM

- Keep the existing Ed25519 signer if supported by the interoperability target, but harden it and wire it to tenant-managed configuration.
- Replace the single filesystem selector assumption with a managed tenant/domain key lifecycle abstraction. Store private key material in a KMS/secret-manager integration or tightly controlled projected secret volume; do not store private key material in PostgreSQL or ordinary config.
- Add create, publish, verify, activate, rotate, retire and revoke operations. Activate only after checking the published DNS public key matches the private key and a defined DNS propagation/TTL procedure has passed.
- Support key selection by verified tenant sending domain and correct From-domain alignment. Prevent a tenant from signing another tenant's domain.
- Consider RSA-SHA256 compatibility only after assessing actual receiver requirements and library support; do not add obsolete RSA-SHA1.
- Test signature validation by an independent DKIM verifier against raw outbound messages and representative receivers, including folded headers, multipart MIME, header order, body newline handling, pre-existing signatures and message mutations.
- Never modify signed body bytes after signing. Define the order of trace headers, platform `Authentication-Results`, DKIM signing and final SMTP wire framing.

## D4. Honest delivery status

Expose precise statuses such as queued, attempting, temporarily deferred, accepted by remote MX, permanently rejected, suppressed, uncertain and delivery-report received. Never label `250` as “delivered to inbox.” UI and API language must explain that remote SMTP acceptance does not guarantee spam-folder placement or mailbox visibility.

Acceptance tests:
- A disposable fake MX/SMTP server covers normal delivery, temporary 4xx, permanent 5xx, TLS required/available/unavailable, invalid certificate, Null MX, MX fallback, safe IP validation, remote disconnect before DATA, disconnect after DATA, lease loss and DB outage.
- Test relay mode with a fake authenticated TLS relay and local Mailpit only in development.
- Test per-recipient mixed success; retry only retryable recipients and do not resend already accepted recipients.
- Test delivery concurrency/rate limits shared across two workers.

---

# PART E — COMPLETE INBOUND MTA AND AUTHENTICATION PIPELINE

Reuse `smtp-session/src/inbound` and current mail-auth integration.

## E1. Finish inbound reliability

- Reconcile the documented event-consumer state with `mailbox-service/src/inbound_projection.rs` and the actual service entry point.
- Ensure every accepted inbound message has durable, idempotent outbox events for all mailbox copies and every downstream side effect.
- Add recovery for pending/expired event leases, bounded retry/backoff, dead-letter/quarantine visibility, replay tooling and reconciliation reports.
- Avoid a second copy/duplicate when an event is delivered more than once or a service crashes between projection and acknowledgment.
- Address SMTP retry idempotency: the current content/envelope fingerprint cannot distinguish every intentional byte-identical re-submission from a sender retry. Define and document a safe deduplication window/identity contract; add a transaction identifier where it can be supplied without breaking SMTP interoperability. Do not claim exactly-once semantics.
- Add explicit operational health for Postgres, scanner, DNS authentication and queue/outbox dependencies.
- Keep scanner outages fail-closed for public inbound according to documented policy; return a temporary SMTP response rather than accepting unscanned messages if scanning is required.
- Add bounded attachment extraction/scanning only through a maintained parser/scanner and isolated processing. Never execute attachments. Do not expand archives without strict decompression and recursion limits.
- Add tenant-specific policy for quarantine, spam score thresholds, malware rejection and retention. Provide audit-safe operator override only with authorization.

## E2. SPF, DKIM and DMARC correctness

The repository already integrates inbound SPF, DKIM and DMARC. Do not replace it with simplified custom parsers. Complete the conformance work:

- Verify RFC 7208 lookup limits, include/redirect/exists/macros, void lookups, permerror/temperror, null reverse path, HELO identity and correct result propagation.
- Verify DKIM RFC 6376 canonicalization and signature validation plus current cryptographic algorithm guidance (RFC 8301/RFC 8463). Test multiple signatures, malformed tags, DNS key records, revoked/expired/weak keys, body hash, header oversigning and key rotation.
- Verify current DMARC standard and alignment/policy behavior (RFC 9989), plus aggregate reporting format/semantics (RFC 9990). Do not rely solely on obsolete RFC 7489-era behavior.
- Add explicit shared DNS query budget accounting across SPF/DKIM/DMARC, rather than depending only on separate library limits and a global timeout.
- Apply DNSSEC and resolver behavior correctly, including CNAME, timeout, NXDOMAIN and SERVFAIL. Distinguish temperror from permerror.
- Prevent a sender from forging platform `Authentication-Results`. Only trusted platform-authentication headers inserted at a trusted boundary may be used by downstream components. Preserve untrusted pre-existing headers without treating them as authoritative.
- Integrate results with quarantine/reject/accept policy as configured and persist safe, structured results with provenance.
- Add tenant-managed DKIM key lifecycle/control-plane API and domain DNS diagnostics for MX, SPF, DKIM selectors, DMARC, MTA-STS, TLS-RPT and ownership. DNS diagnostics are advisory until the record has been independently verified.
- Add RFC-indexed conformance fixtures and real synthetic-domain end-to-end tests.

ARC, BIMI, DANE and S/MIME/OpenPGP are optional future capabilities. Do not let these distract from correct SMTP acceptance, transport, DKIM, SPF and DMARC.

---

# PART F — BOUNCES, DSN, SUPPRESSION AND FEEDBACK

A functioning email service must manage messages that cannot be delivered after acceptance.

- Implement durable per-recipient DSN processing and generation using RFC 3461/3463/3464 and applicable updates.
- Use a null reverse-path (`MAIL FROM:<>`) for generated DSNs to prevent bounce loops.
- Generate DSN only after a message has crossed the durable acceptance boundary and a later delivery failure becomes known.
- Preserve original Message-ID and safe original-recipient metadata; do not echo message bodies or secrets into a DSN unnecessarily.
- Classify remote failures into temporary, permanent, policy, address, mailbox, reputation, TLS and uncertain categories using SMTP response/enhanced status codes, not brittle message text alone.
- Create tenant-scoped hard-bounce and soft-bounce rules, complaint/abuse feedback ingestion, user unsubscribe state and suppression lists.
- Check suppression before enqueue and again before final send where race safety requires it.
- Make bounce/complaint ingestion idempotent and authenticated. Provider feedback webhooks must verify signatures, timestamps and replay windows.
- Add suppression reasons, event sources, timestamps, reactivation rules and audited operator overrides.
- Keep delivery history and API read models persisted and queryable by tenant and authorized user.
- Add customer-configurable webhooks from a durable transactional outbox. Sign payloads with per-tenant secret, use bounded retries/backoff, idempotency IDs, delivery logs and secret rotation. Never call external webhook URLs synchronously inside SMTP acceptance transactions.

Acceptance tests must prove null reverse path, no bounce loops, replay protection, mixed-recipient outcomes, suppression behavior and webhook retry/idempotency.

---

# PART G — MAILBOX, IMAP, STORAGE AND RETENTION

This project includes mailbox/cloud-mail aspirations, so complete the data path without confusing it with SMTP MTA requirements.

## G1. IMAP

- Preserve the existing IMAP4rev1 server and PostgreSQL mailbox store.
- Build interoperability tests for the commands actually advertised and used: LOGIN/AUTHENTICATE PLAIN, LIST/LSUB, STATUS, SELECT/EXAMINE, FETCH, STORE, SEARCH, COPY/MOVE, APPEND, EXPUNGE, UID operations and IDLE.
- Verify UIDVALIDITY/UIDNEXT, sequence-number consistency, concurrent sessions, flag mutation, mailbox change journal ordering, selected-folder invalidation, APPEND literal bounds, partial FETCH, command parsing, tagged response grammar, IDLE notifications and disconnect handling.
- Use TLS-only authentication and the same verified user/mailbox authorization. Never allow tenant-admin role alone to read another user's mailbox without an explicit permission grant.
- Test with at least one independent IMAP test harness and a real desktop/mobile client in a documented manual acceptance matrix. Do not claim full IMAP conformance from unit tests alone.
- Implement additional IMAP extensions only after their semantics and interoperability tests exist. Keep unimplemented features out of CAPABILITY.
- Add operational metrics, readiness, connection draining, TLS certificate reload and per-account/device abuse limits.

## G2. POP3 scope decision

`smtp-pop3` is not a running POP3 service. The existing parser/session is not an acceptable production POP3 implementation.

Either:
- implement a real RFC 1939 POP3 server, with TLS according to RFC 8314, real tenant/account authentication, shared mailbox read adapter, stable message numbering/UIDL, RETR/TOP/DELE/RSET/STAT/LIST/NOOP/QUIT semantics, bounded multiline dot-stuffing, transaction/update behavior, deletion semantics, quotas, limits and integration tests; or
- explicitly mark POP3 out of scope for the first production release and ensure no docs/UI/config imply it is available.

Do not return fake `+OK` responses for operations that do not access real mailbox data.

## G3. Object storage and message lifecycle

- Introduce a storage adapter for large raw messages and attachments, with a local test adapter and a production S3-compatible/provider-neutral implementation once a deployment target is selected.
- PostgreSQL should store durable metadata, tenant ownership, object references, size, digest, encryption metadata, MIME projection, retention deadline and lifecycle state; large blobs should not grow inline without an explicit workload justification.
- Use tenant-scoped encryption and least-privilege service access. Protect object keys and signed URLs; never expose bucket paths as authorization.
- Implement upload/append atomicity, temporary upload cleanup, orphan reconciliation, checksum verification, object availability retries and database/object consistency repair.
- Implement scheduled message/attachment retention, user delete/trash/expunge, tenant policy, legal hold, tombstone, hard-delete workflow, audit, quota release and recovery semantics.
- Ensure legal-hold records block automated deletion, but never claim regulatory compliance without legal/policy review.
- Implement backup/restore of database plus object metadata/content with integrity checks, testable restores and documented RPO/RTO. The current append-idempotency cleanup is not a message-retention system.

---

# PART H — PRODUCTION OPERATIONS AND SECURITY

## H1. Deployable service topology

Document and validate a deployment-ready topology without assuming that a cloud account, region or provider already exists.

Minimum services and boundaries:
- SMTP inbound MTA (port 25).
- SMTP authenticated submission (ports 587 and/or 465).
- Outbound delivery workers.
- Go control-plane API.
- Rust mailbox service.
- IMAP server (993) if hosted mailboxes are in release scope.
- PostgreSQL with backup, restore, migration and connection-pool controls.
- Object storage if hosted message volume justifies the adapter.
- Scanner services for public inbound.
- Centralized logs, metrics, traces and alerting.

Produce explicit configuration, container health checks, resource requests/limits, network boundaries, least-privilege roles and documented scaling. Avoid exposing PostgreSQL, internal mailbox APIs, scanner daemons or admin ports to the public Internet.

## H2. DNS and public deliverability runbook

Document and provide a preflight validator for a domain's required records and operational prerequisites:
- MX and A/AAAA for inbound mail hosts.
- Correct static public egress IP and matching PTR/rDNS for sending hostnames.
- Forward-confirmed reverse DNS where required by receivers.
- SPF record with bounded include/lookup complexity and no unsafe duplicate records.
- DKIM selector records matching the active tenant key.
- DMARC policy and authorized aggregate report destinations.
- MTA-STS policy host/HTTPS certificate and `_mta-sts` record where enabled.
- TLS-RPT `_smtp._tls` record where enabled.
- Domain ownership verification and sender authorization.
- Postmaster and abuse operational contacts/runbooks where required.
- Outbound TCP/25 provider restrictions and tested relay fallback where direct MX is unavailable.

Do not promise inbox placement or sender reputation. Include a monitored warm-up/rate-limit procedure and provider feedback loop readiness, but do not automatically raise sending volume without policy controls.

## H3. Observability and operations

- Replace process-local-only production metrics with scrapeable, fleet-aggregatable metrics and documented dashboards/alerts.
- Provide low-cardinality metrics for active SMTP connections, rejected/accepted sessions, queue depth/oldest age, delivery attempts/outcomes, per-recipient status, temporary/permanent failure rate, DNS latency/errors, STARTTLS rates/failures, certificate expiry, DKIM signing failures, SPF/DKIM/DMARC result categories, inbound scanner latency/availability, mailbox/object-store usage, outbox lag, retries, dead letters, database pool saturation and API auth failures.
- Do not label metrics by email address, recipient, message ID, raw URL, user ID, API key or tenant ID unless a safe bounded aggregation strategy has been explicitly designed.
- Use structured logs and OpenTelemetry-compatible trace propagation across API, enqueue, queue claim, SMTP session, delivery attempt, mailbox projection, bounce and webhook delivery. Correlation IDs must not leak secrets.
- Add SLO definitions, alert thresholds, on-call response steps and runbooks for queue backlog, provider failures, TLS errors, DNS incidents, abuse, scanner outage, DB outage and regional/site outage.
- Add certificate/key renewal monitoring and safe reload tests.
- Add database migration rollback/recovery guidance and safe backup/restore procedures.

## H4. Supply chain and security controls

- Resolve all actionable `cargo audit` and `govulncheck` findings; update deprecated/unmaintained crates with compatible alternatives. Do not suppress advisories without a documented risk acceptance and expiry.
- Add dependency pin/review, SBOM, build provenance, container image scanning, secret scanning, workflow permissions and release artifact verification.
- Ensure production startup rejects default/dev credentials, HTTP public origins, insecure cookies, development auth, test SMTP endpoints, broad outbound allowlists, absent required scanners, absent required TLS and missing production secrets.
- Add a documented GitHub branch-protection/review/release checklist; code alone cannot configure repository protection if deployment permission is unavailable.
- Add rate-limit, tenant isolation, SSRF, SMTP relay, MIME parser, DNS rebinding and message-content privacy security tests.

---

# PART I — CONFORMANCE, INTEGRATION, LOAD AND RECOVERY TESTING

Build on the existing `docs/standards/conformance-testing-strategy.md` and `docs/validation/` rather than inventing a new unmaintainable test system.

Required automated test layers:

1. **Unit tests:** SMTP state machine, path parser, reply parser, MIME parser bounds, DNS records, SPF/DKIM/DMARC results, DKIM key lifecycle, retry classification, tenant authorization and suppression decisions.
2. **Property/fuzz tests:** SMTP command and reply framing, CRLF, DATA dot transparency, ESMTP parameters, MIME nesting/decoding, IDNA/Unicode normalization if enabled, DNS responses, DKIM header parsing, DSN parser and webhook signatures.
3. **PostgreSQL integration:** migrations up/down on disposable DB, queue idempotency, concurrency, fencing, per-recipient outcomes, inbound fanout atomicity, outbox replay, quota, auth sessions, shared rate limits, object references and retention.
4. **SMTP end-to-end:** Python `smtplib`, `swaks`, `openssl s_client`, test MX peers, relay peers, STARTTLS, implicit TLS, AUTH, incoming MTA, submission, durable `250`, retries, bounces and uncertain post-DATA behavior.
5. **IMAP/POP3 integration:** independent protocol harness/client matrix only for protocols marked in scope.
6. **Security tests:** no open relay, tenant isolation, credential stuffing, token leakage, OAuth replay, CSRF, request smuggling, DNS rebinding/SSRF, malicious MIME, scanner failure and secret/log redaction.
7. **Load/soak:** concurrent connections, slow clients, tenant fairness, queue backlog, high-volume DNS lookups, bounded CPU/memory, Postgres saturation and mailbox object throughput.
8. **Chaos/recovery:** kill API, SMTP ingress, worker and mailbox service during key transaction boundaries; Postgres/scanner/DNS/object-store outage; queue lease expiry; cert rotation; backup restore; duplicate event replay.

Every accepted requirement must record the RFC/section (where relevant), test fixture/version, environment, expected protocol result, actual result and date. A single test suite does not prove universal RFC compliance.

Preserve and run at minimum the repository's existing validation:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo test --workspace --doc
cargo build --workspace --all-targets

cd smtp-api
test -z "$(gofmt -l .)"
go test ./...
go test -race ./...
go vet ./...
go build ./...
cd ..

pnpm install --frozen-lockfile
pnpm lint:dashboard
pnpm typecheck:dashboard
pnpm build:dashboard

docker compose config --quiet
./scripts/validate-migrations.sh
```

Use disposable test databases and synthetic mail only. Do not run destructive tests or migration experiments against production data. Update CI so new conformance and integration tests run automatically and cannot be silently skipped.

---

# PART J — REQUIRED IMPLEMENTATION ORDER

Execute in small, independently testable milestones. Do not attempt a giant rewrite.

## Milestone 0 — Repository evidence and P0 security

1. Create the code-backed gap audit.
2. Fix signup verification secret exposure and durable email delivery.
3. Replace process-local production authentication rate limits.
4. Harden OAuth/OIDC callbacks, session lifecycle and JWT key management.
5. Implement trusted SMTP `Received` trace handling without breaking raw-byte/idempotency/DKIM semantics.
6. Add regressions and update security docs.

## Milestone 1 — SMTP listener role separation and correctness

1. Explicit inbound port 25 and submission 587/465 listener profiles.
2. No open relay and correct tenant/domain/recipient authorization.
3. Correct SMTP transaction, acceptance and post-acceptance failure semantics.
4. Runtime-dependent EHLO capabilities.
5. Add PIPELINING only after deterministic ordered response behavior is tested.

## Milestone 2 — Finish outbound MTA M05

1. Per-tenant/domain/IP rate limits and circuit breakers.
2. DNS cache and destination safety review.
3. Tenant/domain TLS policy plus MTA-STS.
4. TLS-RPT reporting; DANE as an independently gated extension.
5. Delivery telemetry, alerts, external MX interoperability tests and safe rollout controls.
6. Validate direct MX, authenticated relay and local Mailpit are isolated by mode.

## Milestone 3 — Finish inbound M06/M07

1. Reliable inbound outbox/projection/recovery and reconciliation.
2. Explicit DNS lookup budgets and RFC-indexed SPF/DKIM/DMARC conformance.
3. Tenant-managed DKIM key lifecycle and independent signing verification.
4. Attachment scanning/object storage integration and retention dependencies.
5. Domain readiness diagnostics and public inbound runbooks.

## Milestone 4 — Delivery outcome, DSN, bounce and suppression

1. Durable delivery read models.
2. DSN generation/ingestion and bounce classification.
3. Feedback/complaint ingestion and suppression.
4. Authenticated, retryable tenant webhooks.

## Milestone 5 — Hosted mailbox completeness

1. IMAP conformance and interoperability matrix.
2. Object storage adapter and message/attachment lifecycle.
3. Retention, deletion, legal-hold and quota reconciliation.
4. POP3 implementation only if it is an explicit product requirement; otherwise mark out of scope.

## Milestone 6 — Release readiness and operations

1. Deployable topology, secret handling and public DNS/send prerequisites.
2. Central telemetry, dashboards, alerts, SLOs and runbooks.
3. Restore/DR and load/chaos evidence.
4. Dependency/security remediation and supply-chain checks.
5. Final release gate with signed evidence; no unsupported “production-ready” claim.

If one milestone is too large, split it into commits/subtasks but preserve this dependency order. Do not skip the P0 fixes to work on dashboard cosmetics or AI features.

---

# PART K — DOCUMENTATION AND CONTRACT CONSISTENCY

After each implementation milestone, reconcile and update, as applicable:

- `README.md`
- `docs/project-status.md`
- `docs/phase-status.md`
- `docs/phase-integration.md`
- `docs/architecture/inbound-mail.md`
- `docs/architecture/outbound-delivery.md`
- `docs/standards/protocol-feature-map.md`
- `docs/standards/conformance-testing-strategy.md`
- `docs/operations.md`
- `docs/testing.md`
- `docs/development/` guides
- `.env.example` and configuration reference
- `docs/api/openapi.yaml` and API docs
- IMAP/POP3 service READMEs

Rules:

- Document current behavior based on source and tests, not old phase plans.
- Resolve contradictions such as the inbound projection consumer being described as both implemented and unwired, and IMAP as both present and pending.
- Reconcile older auth descriptions with existing signup/password, Google/GitHub OAuth and OIDC routes; distinguish code existence from production-ready lifecycle.
- Reconcile outbound relay documentation with the actual authenticated relay implementation/configuration.
- Give every feature a clear status and a link to runtime source/tests.
- State operational prerequisites and unsupported behavior prominently.
- Never claim RFC compliance, inbox placement, HA, security certification or regulatory compliance without appropriate evidence.

---

# PART L — ACCEPTANCE GATES (MUST ALL PASS FOR A PRODUCTION CANDIDATE)

Do not declare production readiness until evidence demonstrates all gates applicable to the chosen release scope.

## SMTP submission

- Python `smtplib` and `swaks` can submit via port 587 with STARTTLS + AUTH and via port 465 with implicit TLS.
- Plaintext AUTH is rejected.
- Authenticated tenant A cannot send using tenant B's domain or credentials.
- Queue acceptance is durable; `250` is not returned before commit.
- Invalid/expired/revoked SMTP credentials are rejected with safe errors and shared rate limiting.

## Inbound SMTP

- External unauthenticated sender can deliver only to verified local recipients via the port 25 profile.
- External-to-external relay is denied.
- SPF/DKIM/DMARC and scanner policies execute under configured bounds.
- A valid trace `Received` header is prepended and existing headers remain intact.
- `250` is sent only after raw message, envelope, routing, quota and outbox state are committed.
- Database/scanner failure returns the correct temporary SMTP response; no silent acceptance or data loss.

## Outbound delivery

- Direct MX and authenticated relay paths are both tested and isolated.
- MX/Null MX/DNS failure/TLS policy behaviors are correct.
- DKIM is applied with an active tenant-owned key and independently verified.
- Each recipient's acceptance/permanent/temporary/uncertain outcome is persisted.
- Accepted recipients are never resent merely because another recipient in the same message failed.
- Retries are bounded, jittered and safe; ambiguous post-DATA outcomes are not blindly retried.
- Bounce/DSN and suppression lifecycles work where those features are included in release scope.

## Identity and tenancy

- Signup/verification/login/recovery/session/OAuth flows pass abuse, replay, CSRF, enumeration and token-leak tests.
- Shared rate limits behave consistently across application instances.
- Tenant isolation tests cover APIs, SMTP credentials, sender authorization, queue, key lifecycle, mailbox, storage, reports and webhooks.
- Secrets are not returned or logged except explicitly one-time credentials where the API contract requires them.

## Storage and operations

- Mailbox content/attachments are stored through the intended durable storage adapter when hosted mail is in scope.
- Retention, deletion, quota accounting and legal hold are exercised end-to-end.
- Backup restore is demonstrated, not just backup file creation/checksum.
- Health/readiness, metrics, alerts, service restart, graceful drain and dependency recovery are tested.
- Deployment prerequisites (DNS, rDNS, TLS, scanners, egress, secret manager, DB backups) are documented and validated.

## Scope honesty

- IMAP is only described as available for the exact tested capability profile.
- POP3 is explicitly unimplemented/out-of-scope until a real server exists.
- SMTPUTF8, DSN, PIPELINING, CHUNKING, MTA-STS, TLS-RPT and DANE are not advertised until their end-to-end tests pass.
- Remote SMTP acceptance is never labeled as inbox delivery.
- AI and multi-region/global infrastructure are not prerequisites for a correct first email-service release.

---

# FINAL COPILOT EXECUTION INSTRUCTIONS

Start by inspecting the current branch and working tree. Do not overwrite unrelated user changes. Create a feature branch and work incrementally.

1. Produce `docs/audits/smtp-production-gap-audit.md` with source/test evidence.
2. Implement Milestone 0 completely before moving to feature expansion.
3. For each milestone, make code changes, additive migrations, focused tests, integration tests, configuration updates and documentation in the same change set.
4. Run the relevant tests after every meaningful change. Run the full repository validation before presenting the result.
5. Review the final diff for accidental secrets, unsafe defaults, tenant leaks, open-relay paths, unsupported EHLO/IMAP capabilities, unbounded memory/CPU, silent error swallowing and stale docs.
6. Do not mark blocked cloud/provider tasks as implemented. Document exact external prerequisites and leave them gated.
7. Provide a final implementation report containing:
   - What was changed, with paths.
   - Which audit findings were fixed and which remain.
   - Migrations added and rollback behavior.
   - Commands/tests run and actual results.
   - Security/conformance limitations.
   - Production deployment prerequisites not verifiable in a code-only environment.
   - A clear release decision: `NOT READY`, `CONTROLLED BETA`, or `PRODUCTION CANDIDATE`, with evidence.

**The objective is a reliable, secure, standards-correct email system—not merely a successful build, an attractive dashboard, a large number of phase documents, or a service that accepts messages but cannot safely deliver and recover them.**

---

# PRIMARY STANDARDS REFERENCES

Use the current normative documents and update/errata links when implementing or testing a capability. Do not use an old phase-plan citation as the source of truth when a newer RFC supersedes it.

- [RFC 5321 — Simple Mail Transfer Protocol](https://www.rfc-editor.org/rfc/rfc5321.html) — SMTP dialogue, acceptance boundary, trace/`Received` headers, relay and failure semantics.
- [RFC 6409 — Message Submission for Mail](https://www.rfc-editor.org/rfc/rfc6409.html) — separate MSA submission policy from MTA relay behavior.
- [RFC 8314 — Cleartext Considered Obsolete: Use of TLS for Email Submission and Access](https://www.rfc-editor.org/rfc/rfc8314.html) — secure submission and mail-access transport.
- [RFC 4954 — SMTP Service Extension for Authentication](https://www.rfc-editor.org/rfc/rfc4954.html) — SMTP AUTH.
- [RFC 3207 — SMTP Service Extension for Secure SMTP over TLS](https://www.rfc-editor.org/rfc/rfc3207.html) — STARTTLS.
- [RFC 2920 — SMTP Service Extension for Command Pipelining](https://www.rfc-editor.org/rfc/rfc2920.html) — PIPELINING.
- [RFC 3030 — SMTP Service Extensions for Transmission of Large and Binary MIME Messages](https://www.rfc-editor.org/rfc/rfc3030.html) — CHUNKING/BDAT and BINARYMIME.
- [RFC 6531 — SMTP Extension for Internationalized Email](https://www.rfc-editor.org/rfc/rfc6531.html) and [RFC 6532](https://www.rfc-editor.org/rfc/rfc6532.html) — SMTPUTF8 and internationalized message headers.
- [RFC 3461](https://www.rfc-editor.org/rfc/rfc3461.html), [RFC 3463](https://www.rfc-editor.org/rfc/rfc3463.html), and [RFC 3464](https://www.rfc-editor.org/rfc/rfc3464.html) — DSN request and report semantics.
- [RFC 7208 — Sender Policy Framework](https://www.rfc-editor.org/rfc/rfc7208.html) — SPF.
- [RFC 6376 — DomainKeys Identified Mail](https://www.rfc-editor.org/rfc/rfc6376.html), [RFC 8301](https://www.rfc-editor.org/rfc/rfc8301.html), and [RFC 8463](https://www.rfc-editor.org/rfc/rfc8463.html) — DKIM and cryptographic updates.
- [RFC 9989 — DMARC](https://www.rfc-editor.org/rfc/rfc9989.html) and [RFC 9990 — DMARC Aggregate Reporting](https://www.rfc-editor.org/rfc/rfc9990.html) — current DMARC specification and aggregate reporting. RFC 9990 (published May 2026) obsoletes RFC 7489; do not implement solely against the old RFC 7489 text.
- [RFC 8461 — SMTP MTA Strict Transport Security](https://www.rfc-editor.org/rfc/rfc8461.html) and [RFC 8460 — SMTP TLS Reporting](https://www.rfc-editor.org/rfc/rfc8460.html) — MTA-STS and TLS-RPT.
- [RFC 7672 — SMTP Security via Opportunistic DANE TLS](https://www.rfc-editor.org/rfc/rfc7672.html) — DANE when DNSSEC validation is implemented and enabled.
- [RFC 8601 — Message Header Field for Indicating Message Authentication Status](https://www.rfc-editor.org/rfc/rfc8601.html) — trusted Authentication-Results handling.
- [RFC 9051 — Internet Message Access Protocol (IMAP) Version 4rev2](https://www.rfc-editor.org/rfc/rfc9051.html), [RFC 3501 — IMAP4rev1](https://www.rfc-editor.org/rfc/rfc3501.html), [RFC 1939 — POP3](https://www.rfc-editor.org/rfc/rfc1939.html), and [RFC 8314](https://www.rfc-editor.org/rfc/rfc8314.html) — mailbox protocol scope and secure transport.

For every requirement, check the RFC Editor page for updates, obsoletes, errata and status at implementation time. Maintain a repository-local RFC-to-test traceability matrix.
