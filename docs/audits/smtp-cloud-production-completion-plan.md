
# SMTP Cloud Platform — Production Completion Plan

**Branch:** smtp-cloud-production-completion-2026-11-02 | **Audit:** 2026-11-02 | **Scope:** Milestones A-O
**Source of truth:** current executable Rust/Go behavior + live migrations override stale docs.

This is the authoritative tracker for every code-level requirement across MILESTONE A-O. Each row records: exact file locations, classification, dependencies, implementation order, migrations, security/reliability implications, test strategy, and verifiable acceptance criteria. ONLY VERIFIED counts as complete.

---
## How to read this document
- Classification: IMPLEMENTED / PARTIAL / MISSING / DOC-ONLY / BLOCKED-EXTERNAL
- Implementation order = dependency-ordered milestone sequence A->O
- Migration = additive PostgreSQL migration number(s); NEVER rewrite deployed migration history
- Security/Reliability = tenant isolation, durability, fail-closed, audit
- Tests = unit/integration/interop/fuzz/chaos with runnable commands
- Acceptance = verifiable evidence text required before VERIFIED

---
## Milestone A — Security and Identity
**Goal:** Complete registration, verification, login/logout, reset, resend, change; Google/GitHub OAuth; generic OIDC; PKCE/state/nonce/REDIRECT validation; secure JWT; persistent opaque sessions; refresh rotation; replay; revocation; expiration; MFA (TOTP + recovery codes); WebAuthn/passkeys where supported; durable provisioning; PostgreSQL-backed auth with abuse limiting; throttling; enumeration-resistant; CSRF; account recovery; encrypted notification outbox; security audit events.

| # | Req | File(s) | Classification | Deps | Migrate | Security/Reliability | Tests | Acceptance |
|---|---|---|---|---|---|---|---|---|
| A1 | Password registration, verification, login, logout, reset, resend, change | password_auth_handlers.go, *_postgres.go, migrations 000029/033/034 | PARTIAL | | M00/M033/M034 | Argon2id; single-use expiring tokens; encrypted outbox; fail-closed | go test ./internal/controlplane | Email link never in response; rotation wipes sessions; reset returns generic error |
| A2 | Google OAuth + GitHub OAuth identity lookup | social_oauth.go, smtp-auth | PARTIAL | A1, OIDC config | | Provider identity bounds; revoke prior sessions | go test ./internal/controlplane | ID token verified; account linked only via exact iss/sub |
| A3 | Generic OIDC integration + provider config | smtp-auth/oidc.go, API OIDC | PARTIAL | | | Discovery at startup; strict URL validation | go test ./... | Discovery verified; iss/aud/exp/iat bounds enforced |
| A4 | PKCE, state, nonce, redirect URI validation, expiry, one-time flow records | oidc.go, migration 000037 | IMPLEMENTED | A2/A3 | M0037 | Hashed state/nonce/verifier; 5-min expiry; one-time consume | go test + replay test | Flow reusable 1x; expired/replayed/cross-provider rejected |
| A5 | Secure JWT verification; strict algorithm allowlist; issuer/audience; kid rotation; revocation; expiration | jwt.go, smtp-auth | PARTIAL | | | HS256 allowlist; iss/aud/exp/iat bounds; kid rotation | go test ./internal/controlplane | Revoked/rejected expired/auid-mismatch |
| A6 | Persistent opaque browser sessions; refresh rotation; replay; revocation; expiration; device mgmt | auth_sessions_handlers.go, auth_sessions_postgres.go, 000028 | PARTIAL | A1/A5 | M028 | CSRF sync-token + same-origin; opaque hashed; idle/absolute TTL; per-device | go test + session lifecycle test | Rotation on login; stolen token detectable; expired rejected |
| A7 | MFA TOTP + recovery codes | smtp-auth/totp.go, totp_test.go | NEW(in prog) | | | TOTP time/drift windows; 160-bit secret; one-time recovery codes | go test ./smtp-auth + client test | 2FA prompt; codes rotate; recovery invalid after use |
| A8 | WebAuthn/passkeys where supported | smtp-auth | MISSING | A7 | | Credential ID uniqueness; public key storage | | Not in scope for first release if no provider; documented |
| A9 | Durable identity provisioning + tenant memberships | *_postgres.go, domains_postgres.go | IMPLEMENTED | A | M00-M01 | Ownership constraints; role enforcement; tenant-active join | go test ./internal/controlplane | New tenant auto-provisions owner; membership roles persisted |
| A10 | PostgreSQL-backed distributed auth abuse limiting | rate_limit.go, auth_postgres.go | BROKEN for production | A1/A10 | | Atomic bounded keys; HMAC pseudonymization; fail closed; local limits dev/test | go test -race; load test | Shared limiter across replicas; unavailable blocks auth |
| A11 | Login throttling, credential stuffing, enumeration-resistant, CSRF, account recovery | password_auth_handlers.go, auth_middleware.go | PARTIAL | A10 | | Fixed generic rate-limit errors; no enumeration; CSRF; recovery w/ MFA context | go test ./internal/controlplane | Same login error; verification indistinguishable; CSRF rejected |
| A12 | Encrypted transactional notification outbox w/ retry, dead-letter, replay | auth_email_outbox.go, 000034 | IMPLEMENTED | A1 | M034 | Encrypt at rest; outbound consumer; lease/retry; dead-letter; replay | go test + outbox consumer test | Consumed exactly-once; dead-letter replayed; no in-memory outbox |
| A13 | Complete security audit events | store.go | PARTIAL | A1-A12 | | Audited auth events; tenant-scoped; tamper-evident; export | go test ./internal/controlplane | Audit rows survive restart; ordered; actor/target present |

**Order:** A1 -> A2 -> A3 -> A5 -> A6 -> A7 -> A10 -> A11 -> A12 -> A13
**Acceptance:** cd smtp-api && go test ./internal/controlplane/... -race -count=1


## Milestone B — Control Plane and Multi-Tenancy
**Goal:** PostgreSQL authoritative state for users/tenants/memberships/roles, domain ownership/verification, API keys/scopes/expiration/rotation/revocation, tenant plans/entitlements/limits/quotas, usage metering & reconciliation, durable audit records, SMTP credentials & lifecycle, DKIM config, mailbox/domain administration, delivery read models & message history. Remove process-local state from production paths.

| # | Req | File(s) | Classification | Deps | Migrate | Test | Acceptance |
|---|---|---|---|---|---|---|---|
| B1 | Users, tenants, memberships, roles in PostgreSQL | *_postgres.go, migrations | MISSING | A | M00-M01 | go test + integration | SQL schema verified; API returns DB rows not in-memory |
| B2 | Domain ownership & verification | domains_postgres.go | PARTIAL | B1 | M00-M01 | go test | Domain pending->verified via DNS TXT; ownership not transferable |
| B3 | API keys, scopes, expiration, rotation, revocation | api_keys_postgres.go | PARTIAL | B1 | M00-M01 | API key integration tests | Rotation invalidates old; revoked/expired rejected |
| B4 | Tenant plans, entitlements, limits, quotas | tenants_*, *_postgres.go | MISSING | B3 | | Go unit | Plan limits enforced; usage in DB not memory |
| B5 | Usage metering & reconciliation | usage_postgres.go | MISSING | B4 | | Integration | Usage returns DB sum; quota exceeded blocks send |
| B6 | Durable audit records | audit_postgres.go | PARTIAL | A13 | | Integration | Audit rows persisted; ordered; tenant-scoped |
| B7 | SMTP credentials & lifecycle | smtp_credentials.go | PARTIAL | B3 | M00-M01 | Go tests | One-time secret; hashed store; revocation invalidates |
| B8 | DKIM configuration | dkim_* + smtp-core | MISSING | B3 | | Unit/integration | DKIM key lifecycle (create/rotate/revoke) works |
| B9 | Mailbox & domain administration | mailboxes_* + mailbox-service | PARTIAL | B1 | | API integration | Mailbox CRUD; permissions; quota enforced |
| B10 | Delivery read models & message history | delivery_events.go | PARTIAL | D | | Go test | Delivery history tenant-scoped; DB-backed |

**Order:** B1 -> B2 -> B3 -> B4 -> B5 -> B7 -> B6 -> B9 -> B8 -> B10
**Acceptance:** cd smtp-api && go test ./internal/controlplane/... -race -count=1


## Milestone C — SMTP Protocol and Ingress
**Goal:** Review SMTP vs RFC 5321/RFC 6409. Implement separate relay, inbound MTA, authenticated submission policies; explicit profiles for 25/587/465; STARTTLS + implicit TLS state reset; secure SMTP AUTH + tenant-scoped authz; connection limits, per-IP/tenant, timeouts, graceful shutdown; strict command parsing, CRLF, DATA framing, dot transparency; correct MAIL FROM/RCPT TO; message size negotiation; secure Received trace; safe responses; transaction reset; robustness for slow clients, malformed, disconnects, TLS failures.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| C1 | Separate relay, inbound MTA, authenticated submission policies | config.rs, main.rs | IMPLEMENTED | cargo test -p smtp-ingress | Profiles validate; bad combos rejected |
| C2 | Explicit listener profiles for 25/587/465 | main.rs | IMPLEMENTED | cargo test -p smtp-ingress | 3 profiles bind correct ports |
| C3 | STARTTLS + implicit TLS state reset | session.rs, server.rs | IMPLEMENTED | cargo test -p smtp-ingress | STARTTLS ok; post-upgrade AUTH works; non-TLS AUTH rejected |
| C4 | Secure SMTP AUTH + tenant-scoped authz | auth/, queue auth | PARTIAL | cargo test -p smtp-ingress | Auth rejected no-TLS; wrong tenant rejected |
| C5 | Connection limits, per-IP/tenant, timeouts, graceful shutdown | config.rs, server.rs | IMPLEMENTED | cargo test -p smtp-ingress | Backpressure; graceful drain |
| C6 | Strict command parsing, CRLF, DATA framing, dot transparency | command.rs | IMPLEMENTED | cargo test -p smtp-ingress | Malformed rejected; dot removed; DATA boundary correct |
| C7 | Correct MAIL FROM / RCPT TO | session.rs | IMPLEMENTED | cargo test -p smtp-ingress | Correct envelope; 500 invalid; 452 limit |
| C8 | Message size negotiation/enforcement | session.rs, config.rs | IMPLEMENTED | cargo test -p smtp-ingress | SIZE advertised; 552 exceed; per-domain enforcement |
| C9 | Secure Received trace generation | inbound/mod.rs | BROKEN | Rust integration test | Received header includes trusted IP; provenance legible |
| C10 | Safe response handling + transaction reset | session.rs | IMPLEMENTED | cargo test -p smtp-ingress | 4xx keeps txn; 5xx resets; invalid handled |
| C11 | Robustness: slow clients, malformed, disconnects, TLS failures | server.rs, session.rs | PARTIAL | Fuzz test; slow-client test | No stuck connections; TLS failure isolated |
| C12 | PIPELINING/SMTPUTF8/CHUNKING/BINARYMIME unadvertised | session.rs | NOT-ADVERTISED | Protocol test | EHLO does not advertise unready extensions |

**Order:** C1->C2->C3->C4->C5->C6->C7->C8->C9->C10->C11->C12
**Rust acceptance:** cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; cargo check --workspace --all-targets; cargo test --workspace --all-targets; cargo test --workspace --doc; cargo build --workspace --all-targets


## Milestone D — Durable Queue and Outbound Delivery
**Goal:** Complete real outbound delivery pipeline: durable PostgreSQL queue; recipient-level records; idempotent enqueue; lease fencing; renewal; retry scheduling; uncertain-outcome; per-tenant fairness; destination/domain concurrency; temp/permanent error classification; MX discovery; null MX; DNS safety; DNS caching; tenant/domain TLS; cert validation; opportunistic TLS policy; destination-IP throttling; circuit breakers; backpressure/queue-age; SMTP lifecycle bounded; per-recipient outcomes; kill switches/suspension; delivery metrics; traces; alerts; recovery.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| D1 | Durable PostgreSQL queue w/ recipient delivery records | store.rs, migrations | IMPLEMENTED | cargo test -p mail-queue | Rows persist after restart |
| D2 | Idempotent enqueue + lease fencing | store.rs | IMPLEMENTED | cargo test -p mail-queue | No duplicate delivery |
| D3 | Lease renewal, retry scheduling, uncertain outcomes | processor.rs, delivery/processor.rs | IMPLEMENTED partial | cargo test -p mail-queue | Expired lease not reprocessed; uncertain retried |
| D4 | Per-tenant fairness + destination/domain concurrency | worker.rs, config.rs | PARTIAL | cargo test -p mail-delivery | Fairness within limit; no runaway domain |
| D5 | Temp/permanent error classification | reply.rs, model.rs | IMPLEMENTED | cargo test -p mail-delivery | Correct classification; no retry permanent |
| D6 | Remote MX discovery, null MX, DNS destination safety | outbound/dns.rs | IMPLEMENTED | cargo test -p mail-delivery | Null MX handled; DNS failures logged, not fatal |
| D7 | DNS caching + connection policy enforcement | dns.rs, worker.rs | IMPLEMENTED | cargo test -p mail-delivery | DNS failures bounded with retries |
| D8 | Tenant/domain TLS requirements + cert validation + opportunistic TLS | outbound/config.rs, client.rs | PARTIAL | cargo test -p mail-delivery | Required TLS enforced for tenants; opportunistic validated |
| D9 | Destination-IP throttling + circuit breakers | worker.rs, config.rs | PARTIAL | cargo test -p mail-delivery | IP throttle prevents runaway; breaker trips |
| D10 | Backpressure + queue-age controls | worker.rs | IMPLEMENTED | cargo test -p mail-delivery | Age thresholds block; backpressure propagates |
| D11 | SMTP connection lifecycle + bounded resources | client.rs | IMPLEMENTED | integration test | Connections bounded; disconnect on failure;
| D12 | Per-recipient outcomes + delivery history | model.rs, delivery_events | IMPLEMENTED partial | integration test | Recipient outcomes persisted; history accurate |
| D13 | Operational kill switches + tenant suspension | config.rs, worker.rs | MISSING | Integration test | Kill switch stops outbound; suspended tenant quarantines |
| D14 | Delivery metrics, traces, alerts, failure recovery | metrics.rs, observability | PARTIAL | cargo test -p mail-delivery | Metrics expose depth/age/outcomes; alerts wired |

**Order:** D1->D2->D3->D4->D5->D6->D7->D8->D9->D10->D11->D12->D13->D14
**Acceptance:** cargo fmt/clippy/check/test/build as C.


## Milestone E — Inbound Mail and Deliverability
**Goal:** Public inbound SMTP listener; verified-domain + mailbox routing; tenant-safe multi-recipient fan-out; atomic acceptance + durable outbox events; trusted Received + Authentication-Results; ClamAV + SpamAssassin; scanner timeout/outage/fail-closed/retry; SPF verification; DKIM verification; DMARC discovery/alignment/enforcement/reporting; explicit DNS query budgets + bounded resolver; safe duplicate detection + intentional resubmission; inbound recovery/replay/dead-letter/reconciliation; monitoring/operational controls.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| E1 | Public inbound SMTP listener | main.rs, config.rs | PARTIAL | cargo test -p smtp-ingress | Inbound enabled binds public port; routing correct |
| E2 | Verified-domain + mailbox routing | inbound/mod.rs | PARTIAL | cargo test -p smtp-ingress | Inbound routed correctly; cross-tenant rejected |
| E3 | Tenant-safe multi-recipient fan-out | inbound/mod.rs | PARTIAL | cargo test -p smtp-ingress | Multi-recipient fan-out atomic; cross-tenant rejected |
| E4 | Atomic acceptance + durable outbox event | inbound/mod.rs, queue | PARTIAL | integration test | Durable outbox event per copy; atomic acceptance |
| E5 | Trusted Received + Authentication-Results | inbound/mod.rs, email-auth | PARTIAL | integration test | Received trusted; Auth-Results correct |
| E6 | ClamAV + SpamAssassin integration | scanner.rs | PARTIAL | cargo test -p smtp-ingress | Scanner outage fails closed; malware/spam rejected |
| E7 | Scanner timeout, outage, retry, fail-closed | scanner.rs | PARTIAL | integration test | Scanner outage returns temp 4xx; success retries cleanly |
| E8 | SPF verification | spf_dkim.rs | IMPLEMENTED partial | unit test | SPF result correct; bounded lookups |
| E9 | DKIM verification | spf_dkim.rs, dkim | IMPLEMENTED partial | unit test | DKIM verified; store results |
| E10 | DMARC discovery, alignment, enforcement, reporting | dmarc-report-* | IMPLEMENTED partial | integration test | DMARC enforced w/ alignment; reports ingested |
| E11 | Explicit DNS query budgets + bounded resolver | dns.rs, resolver | MISSING | integration test | DNS budget enforced; bounded resolver behavior |
| E12 | Safe duplicate detection + intentional resubmission | inbound/mod.rs | PARTIAL | integration test | Duplicate detection correct; resubmission preserved |
| E13 | Inbound recovery, replay, dead-letter, reconciliation | recovery | MISSING | integration test | Recovery/replay/dead-letter works |

**Order:** E1->E2->E3->E4->E5->E6->E7->E8->E9->E10->E11->E12->E13
**Acceptance:** cargo test -p smtp-ingress + inbound integration test (cross-tenant no-leak proof)


## Milestone F — DKIM, Domain Security, DNS
**Goal:** Tenant-managed DKIM lifecycle: secure key generation; encrypted/secret-manager-backed storage; DNS TXT guidance; DNSSEC-aware verification; active selector management; rotation/retirement; revocation & audit history; multi-domain signing/verification; signing interoperability fixtures. MTA-STS, TLS-RPT, DANE as separate capabilities with documented prerequisites + RFC conformance tests.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| F1 | DKIM secure key generation | dkim + smtp-delivery bins | MISSING | unit test | Key generated deterministically fenced; no reuse |
| F2 | Encrypted/secret-manager key storage | dkim + secrets | MISSING | unit test | Keys encrypted at rest; key material never in logs/DB |
| F3 | DNS TXT publication guidance | dkim | IMPLEMENTED partial | API endpoint | DNS TXT instructions returned |
| F4 | DNSSEC-aware verification | domain-readiness + resolver | PARTIAL | unit test | DNSSEC verified before trusting records |
| F5 | Active selector management | dkim | MISSING | API test | Selectors created/rotated; active updated |
| F6 | Key rotation & retirement | dkim | MISSING | integration test | Rotation completes with zero-signing outage |
| F7 | Revocation & audit history | dkim | MISSING | integration test | Revoked key rejected; history persisted |
| F8 | Multi-domain signing & verification | dkim + smtp-core | IMPLEMENTED partial | interop test | Multi-domain signing verified |
| F9 | Signing interoperability fixtures | dkim + tests | IMPLEMENTED partial | cargo test -p smtp-pair | Interop fixture passes |
| F10 | MTA-STS capability | smtp-core + docs | MISSING | RFC conformance test | Verified when configured; prereqs documented |
| F11 | TLS-RPT | smtp-core + dmarc | MISSING | RFC conformance test | TLS-RPT reports generated |
| F12 | DANE | smtp-core + resolver | MISSING | RFC conformance test | DANE evaluated where configured |

**Order:** F1->F2->F5->F6->F7->F3->F8->F9->F4->F10->F11->F12
**Acceptance:** cargo test -p smtp-pair + dkim unit tests


## Milestone G — Mailbox and IMAP
**Goal:** Mailbox lifecycle & tenant membership; shared mailbox permissions; folder hierarchy + special-use; durable append/fetch/flags/deletion/expunge; UID, UIDVALIDITY, UIDNEXT, quota; sync & change tracking; search & pagination; threading & conversation views; draft & sent-copy projection; retention, deletion, legal hold, recovery; durable content storage. Complete IMAP4rev1/IMAP4rev2 server.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| G1 | Mailbox lifecycle & tenant membership | mailbox-service + store | PARTIAL | API + mailbox integration tests | Mailbox CRUD; permissions; membership |
| G2 | Shared mailbox permissions | mailbox-service | MISSING | integration test | Shared mailbox access scoped correctly |
| G3 | Folder hierarchy + special-use folders | store/folders | PARTIAL | integration test | Folders hierarchical; special-use correct |
| G4 | Durable append, fetch, flags, deletion, expunge | store/messages | PARTIAL | integration tests | Append idempotent; flags/updates correct; expunge durable |
| G5 | UID, UIDVALIDITY, UIDNEXT, quota semantics | store | PARTIAL | integration test | UID/UIDVALIDITY/UIDNEXT correct; quota enforced |
| G6 | Mailbox sync + change tracking | sync.rs | PARTIAL | integration test | Change journal durable; sync cursors correct |
| G7 | Search + pagination | search.rs | PARTIAL | integration test | Search returns correct results; pagination bounded |
| G8 | Message threading + conversation views | threading.rs | PARTIAL | integration test | Threads/conversation correct |
| G9 | Draft + sent-copy projection | store/submissions | PARTIAL | integration test | Draft saved; sent-copy projected after delivery |
| G10 | Retention, deletion, legal hold, recovery | store/retention | PARTIAL | integration test | Retention enforces; hold protects; recovery works |
| G11 | Durable content storage | object storage (Milestone I) | MISSING | integration test | Message content in object storage; metadata in DB |
| G12 | IMAP4rev1/IMAP4rev2 server | smtp-imap | PARTIAL | interop tests | All advertised commands implemented correctly; seqnums/UIVALIDITY correct |

**Order:** G1->G3->G4->G5->G6->G7->G8->G9->G10->G2->G11->G12
**Acceptance:** cargo test -p smtp-imap + mailbox integration tests + independent-client test


## Milestone H — POP3 Scope Decision
**Goal:** POP3: either implement real RFC 1939 service OR explicitly exclude from first release and remove misleading claims/config/dashboard controls. Never expose placeholder PASS as real auth.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| H1 | Real RFC 1939 POP3 service OR explicit exclusion | smtp-pop3 | MISSING | cargo test -p smtp-pop3 | Either: real service OR exclusion documented; no placeholder PASS |

**Decision:** Current smtp-pop3 is parser/state-machine fixtures only. First production release: exclude POP3, remove misleading claims/config/UI. Document as BLOCKED-EXTERNAL for later phase.
**Acceptance:** No listener in default config; smtp-pop3 crate documented as fixtures only; any config referencing POP3 removed.


## Milestone I — Object Storage and Attachments
**Goal:** Provider-neutral storage abstraction + real durable S3-compatible: streaming upload/download; multipart upload & abort cleanup; encrypted content; tenant-safe metadata; key versioning & rotation; tenant-scoped dedup; persistent upload sessions; DB/object-store consistency & orphan reconciliation; malware scanning & quarantine; authorized downloads & expiring signed URLs; safe content disposition & bounded range reads; retention, deletion, legal hold integration; quotas, integrity checks, audit events; backup & restore verification. Keep large bodies/attachments out of unbounded memory.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| I1 | Provider-neutral storage abstraction + S3-compatible impl | smtp-storage | MISSING | unit test | Provider-neutral adapter works; S3-compatible |
| I2 | Streaming uploads/downloads | storage | MISSING | unit test | Streaming works; bounded memory |
| I3 | Multipart upload + abort cleanup | storage | MISSING | integration test | Multipart complete/abort cleans up |
| I4 | Encrypted content + tenant-safe metadata | storage + secrets | MISSING | unit test | Encryption works; metadata tenant-scoped |
| I5 | Key versioning & rotation | storage | MISSING | integration test | Versions stored; rotation works |
| I6 | Tenant-scoped dedup | storage | MISSING | unit test | Dedup isolated per tenant |
| I7 | Persistent upload sessions | storage | MISSING | integration test | Sessions durable; expire clean up |
| I8 | DB/object-store consistency + orphan reconciliation | storage + DB | MISSING | integration test | Orphans cleaned; consistency verified |
| I9 | Malware scanning + quarantine | storage + scanner | MISSING | integration test | Malware quarantined; scan results persisted |
| I10 | Authorized downloads + expiring signed URLs | storage | MISSING | integration test | Expiring signed URL works; auth enforced |
| I11 | Safe content disposition + bounded range reads | storage | MISSING | integration test | Content disposition correct; ranges bounded |
| I12 | Retention, deletion, legal hold integration | storage | MISSING | integration test | Retention/deletion/hold consistent across DB+storage |
| I13 | Quotas, integrity checks, audit events | storage | MISSING | integration test | Quota enforced; checksums verified; audit events |
| I14 | Backup & restore verification | storage + scripts | MISSING | integration test | Backup restores; data reconciles |

**Order:** I1->I2->I3->I4->I5->I6->I7->I8->I9->I10->I11->I12->I13->I14
**Acceptance:** cargo test -p smtp-storage + integration tests + backup/restore test


## Milestone J — Bounces, Suppressions, Events
**Goal:** Durable bounce/complaint ingestion; DSN generation & parsing; recipient suppression records; hard-bounce & complaint policy; tenant-level suppression management; enforcement before every outbound enqueue; race-safe suppression updates; delivery event history; durable webhook subscriptions; HMAC-signed webhook payloads; retry schedules, dead-letter, manual replay; endpoint verification & failure isolation; idempotency & event versioning.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| J1 | Durable bounce/complaint ingestion | delivery_events + feedback | IMPLEMENTED partial | go test ./internal/controlplane | Bounce/complaint ingested durably |
| J2 | DSN generation & parsing | smtp-core delivery/dsn | IMPLEMENTED partial | unit test | DSN generated; feedback parsed correctly |
| J3 | Recipient suppression records + tenant-level mgmt | suppressions + api | IMPLEMENTED partial | go test; interop test | Suppression records; tenant admin; enforcement gate |
| J4 | Hard bounce + complaint policy | feedback + queue | IMPLEMENTED partial | integration test | Hard bounce blocks retry; complaint suppresses |
| J5 | Enforcement before every outbound enqueue | queue enqueuing | MISSING | integration test | No enqueue for suppressed recipient; race-safe |
| J6 | Delivery event history | delivery_events | PARTIAL | go test | Delivery history accurate; tenant-scoped |
| J7 | Durable webhook subscriptions | webhook + 000038 | IMPLEMENTED partial | go test | Subscription CRUD; HMAC; signing; stored secret hash |
| J8 | HMAC-signed webhook payloads | webhook_worker | IMPLEMENTED partial | go test | Payload HMAC verified by receiver |
| J9 | Retry schedules, dead-letter, manual replay | webhook_worker | IMPLEMENTED partial | go test | Failed webhook retried; dead-letter replayable |
| J10 | Endpoint verification + failure isolation | webhook | IMPLEMENTED partial | integration test | Dead endpoint isolated; fails over to others |
| J11 | Idempotency + event versioning | webhook + delivery_events | MISSING | integration test | Webhook dedup works; events versioned |

**Order:** J1->J2->J3->J4->J5->J6->J7->J8->J9->J10->J11
**Acceptance:** go test ./internal/controlplane + webhook integration tests


## Milestone K — API and Dashboard Integration
**Goal:** Versioned API + OpenAPI for auth/account, users/teams/roles, tenants/billing, domains/DNS, API keys, SMTP credentials, DKIM, sending/delivery history, mailbox management, messages/folders, attachments, suppressions/bounces, webhooks/event history, usage/quotas, security audit/operations. Backend operations exist for every dashboard feature; remove fake upcoming-feature controls. Sync frontend types with OpenAPI.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| K1 | Auth & account management APIs | password_* + session routes | PARTIAL | go test ./internal/controlplane | Auth/account APIs functional |
| K2 | Users, teams, roles APIs | users/memberships/roles | MISSING | integration test | User/team/role APIs functional |
| K3 | Tenants & billing plans APIs | tenants + plans | MISSING | integration test | Tenant + plan APIs functional |
| K4 | Domains & DNS verification APIs | domains | PARTIAL | integration test | Domain APIs functional |
| K5 | API keys APIs | api_keys | PARTIAL | integration test | API key APIs functional |
| K6 | SMTP credentials APIs | smtp_credentials | PARTIAL | integration test | SMTP credential APIs functional |
| K7 | DKIM settings APIs | dkim | MISSING | integration test | DKIM settings APIs functional |
| K8 | Sending & delivery history APIs | submissions + delivery_events | PARTIAL | integration test | History APIs return real data |
| K9 | Mailbox management APIs | mailboxes | PARTIAL | integration test | Mailbox management APIs functional |
| K10 | Messages & folders APIs | messages + folders | PARTIAL | integration test | Message/folder APIs functional |
| K11 | Attachments APIs | attachments | MISSING | integration test | Attachment APIs functional |
| K12 | Suppressions & bounces APIs | suppressions + bounces | PARTIAL | integration test | Suppression/bounce APIs functional |
| K13 | Webhooks & event history APIs | webhooks + events | PARTIAL | integration test | Webhook APIs functional |
| K14 | Usage & quotas APIs | usage + plans | MISSING | integration test | Usage/quota APIs functional |
| K15 | Security audit & operational APIs | audit + ops | PARTIAL | integration test | Audit/ops APIs functional |

**Order:** K1->K2->K3->K4->K5->K6->K7->K8->K9->K10->K11->K12->K13->K14->K15
**Acceptance:** cd smtp-api && go test ./... -race; all APIs return real data not placeholders


## Milestone L — Search, Retention, Data Lifecycle
**Goal:** Durable search indexing; rebuildable indexing pipelines; tenant-safe search; search lag/reconciliation; message/attachment retention; legal holds; tenant export & deletion; tombstone + async hard deletion; quota release; audited lifecycle events; backup retention & recovery. Ensure deletion propagates to queues, mailbox metadata, object storage, indexes, caches, derived projections.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| L1 | Durable search indexing | search + index store | PARTIAL | integration test | Search index durable |
| L2 | Rebuildable indexing pipeline | index pipeline | MISSING | integration test | Index rebuild complete & resumable |
| L3 | Tenant-safe search queries | search | PARTIAL | integration test | Queries tenant-scoped |
| L4 | Search lag + reconciliation | metrics + monitoring | MISSING | integration test | Lag tracked; reconciliation completes |
| L5 | Message & attachment retention | retention + storage | PARTIAL | integration test | Retention enforced across DB + storage |
| L6 | Legal holds | retention + hold | MISSING | integration test | Hold prevents deletion |
| L7 | Tenant export & deletion | tenant management | MISSING | integration test | Delete removes all data from all systems |
| L8 | Tombstone + async hard deletion | storage + DB | MISSING | integration test | Tombstones respected; hard delete safe |
| L9 | Quota release | mailbox quota | PARTIAL | integration test | Quota correctly adjusted on delete |
| L10 | Audited lifecycle events | audit | PARTIAL | integration test | Lifecycle events audited |
| L11 | Backup retention & recovery | backup scripts | MISSING | integration test | Backup/restore verified |

**Order:** L1->L2->L3->L4->L5->L6->L7->L8->L9->L10->L11
**Acceptance:** integration tests + backup/restore drill


## Milestone M — Observability & Production Operations
**Goal:** OpenTelemetry-compatible metrics, tracing, structured logs across Rust/Go/workers: SMTP command latency/failures; auth rejections & throttling; TLS failures; queue depth/age/lease expiry; per-recipient delivery outcomes; remote SMTP response classes; DNS failures/latency; bounce/complaint rates; scanner health; mailbox growth/search lag; API latency/errors; webhook retries/dead letters; storage capacity/failures. Dashboards, SLOs, alert rules, runbooks, readiness checks, graceful shutdown. Deployment artifacts, container hardening, secret injection, migration strategy, rollback, backup/restore automation, DR exercises.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| M1 | OTel metrics + tracing + structured logs | metrics.rs, observability, API | PARTIAL | unit tests | Metrics/traces exported; structured logs |
| M2 | SMTP command latency & failures | SMTP metrics | MISSING | unit test | Command metrics exported |
| M3 | Auth rejection & throttling | auth metrics | MISSING | integration test | Auth metrics correct |
| M4 | TLS failures | TLS metrics | MISSING | integration test | TLS metrics correct |
| M5 | Queue depth, age, lease expiry | queue metrics | PARTIAL | integration test | Queue metrics correct |
| M6 | Per-recipient delivery outcomes | delivery metrics | PARTIAL | integration test | Delivery outcome metrics correct |
| M7 | Remote SMTP response classes | delivery metrics | PARTIAL | integration test | Response class metrics correct |
| M8 | DNS failures & latency | DNS metrics | MISSING | integration test | DNS metrics correct |
| M9 | Bounce & complaint rates | feedback metrics | PARTIAL | integration test | Bounce/complaint metrics correct |
| M10 | Scanner health | scanner metrics | PARTIAL | integration test | Scanner health tracked |
| M11 | Mailbox growth & search lag | mailbox metrics | MISSING | integration test | Mailbox growth & search lag metrics correct |
| M12 | API latency & errors | API observability | MISSING | integration test | API metrics correct |
| M13 | Webhook retries & dead letters | webhook metrics | PARTIAL | integration test | Webhook metrics correct |
| M14 | Storage capacity & failures | storage metrics | MISSING | integration test | Storage metrics correct |
| M15 | Dashboards, SLOs, alert rules, runbooks | grafana/prometheus | MISSING | — | Deployment artifacts complete |
| M16 | Readiness checks + graceful shutdown | main.rs, server.rs | IMPLEMENTED partial | integration test | Readiness checks + graceful shutdown work |

**Order:** M1->M2->M3->M4->M5->M6->M7->M8->M9->M10->M11->M12->M13->M14->M15->M16
**Acceptance:** metrics exposed; dashboards; alerts; runbooks; deployment tested


## Milestone N — Security & Supply Chain
**Goal:** Resolve RustSec findings; dependency vulnerability scanning; SBOM generation; build provenance & artifact signing; container image scanning; secret scanning; least-privilege GitHub Actions; branch protection & required status checks; release authorization; TLS & crypto config review; threat-model updates; penetration testing plan; fuzzing & protocol abuse testing; tenant-isolation testing; database privilege separation; incident response documentation.

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| N1 | RustSec findings resolved | cargo audit + dependency upgrades | IMPLEMENTED | cargo audit | No unresolved high/critical findings |
| N2 | Dependency vulnerability scanning | CI + tooling | MISSING | CI job | CI scans deps; SBOM generated |
| N3 | SBOM generation | CI + tooling | MISSING | CI job | SBOM in release artifacts |
| N4 | Build provenance & artifact signing | CI + tooling | MISSING | CI job | Provenance signed artifacts in release |
| N5 | Container image scanning | CI + tooling | MISSING | CI job | Image scanned; vulnerabilities remediated |
| N6 | Secret scanning | CI + gitleaks | MISSING | CI job | Secrets detected; no false negatives |
| N7 | Least-privilege GitHub Actions | .github/workflows | MISSING | CI config | Workflow permissions least-privilege |
| N8 | Branch protection & required status checks | GitHub + CI | MISSING | GitHub config | Branch protection enforced |
| N9 | Release authorization | CI + tooling | MISSING | CI job | Releases require approval |
| N10 | TLS/crypto config review | crypto review | PARTIAL | review | TLS 1.2+; strong ciphers; cert validation; OCSP |
| N11 | Threat-model update | security/threat-model | PARTIAL | review | Threat model current |
| N12 | Penetration testing plan | security/security-testing | MISSING | plan | Pen test plan documented |
| N13 | Fuzzing & protocol abuse testing | fuzzing + interop | MISSING | CI job | Fuzzers run; findings tracked/resolved |
| N14 | Tenant-isolation testing | tenant isolation | PARTIAL | integration test | No cross-tenant leakage |
| N15 | Database privilege separation | DB deployment | MISSING | deployment | DB user least-privilege |
| N16 | Incident response docs | security/incident-response | PARTIAL | review | IR docs current |

**Order:** N1->N2->N3->N4->N5->N6->N7->N8->N9->N10->N11->N12->N13->N14->N15->N16
**Acceptance:** CI complete; SBOM; provenance; images scanned; branch protection; DR tested


## Milestone O — Performance & Scale Validation
**Goal:** Measurable service-level targets before optimization: concurrent SMTP connections; message throughput & queue growth; delivery concurrency & remote failures; DB pool saturation; large MIME messages & attachments; mailbox read/write concurrency; search performance; tenant fairness; rate-limit correctness across replicas; worker restarts & lease recovery; storage outages; DNS failures; DB failures & recovery; backup restore & data reconciliation. Run repeatable load/stress/soak/chaos tests. Record hardware/workload/dataset/config + raw results (no invented numbers).

| # | Req | File(s) | Classification | Test | Acceptance |
|---|---|---|---|---|---|
| O1 | Concurrent SMTP connections | session.rs, server.rs | PARTIAL | load test | Connection concurrency verified |
| O2 | Message throughput & queue growth | queue + worker | PARTIAL | load test | Throughput measured; no unbounded growth |
| O3 | Delivery concurrency & remote failures | worker.rs | PARTIAL | load/failure test | Delivery concurrency verified; failures handled |
| O4 | Database pool saturation | DB config | PARTIAL | load test | Pool saturation handled correctly |
| O5 | Large MIME messages & attachments | mime + storage | PARTIAL | load test | Large messages handled; memory bounded |
| O6 | Mailbox read/write concurrency | mailbox-service | PARTIAL | load test | Concurrent ops correct; quota accurate |
| O7 | Search performance | search | PARTIAL | load test | Search latency logged; within SLO |
| O8 | Tenant fairness | worker.rs | PARTIAL | load test | Tenants treated fairly |
| O9 | Rate-limit correctness across replicas | A10 + load | PARTIAL | chaos test | Rate limit works across replicas |
| O10 | Worker restart & lease recovery | D3 | PARTIAL | chaos test | Leases recovered after restart |
| O11 | Storage outage | I | PARTIAL | chaos test | Storage outage handled; no data loss |
| O12 | DNS failure | D6-D7 | PARTIAL | chaos test | DNS failure handled; backoff applies |
| O13 | DB failure & recovery | B1 | PARTIAL | chaos test | DB failover/recovery correct |
| O14 | Backup restore & data reconciliation | backup scripts | MISSING | chaos test | Backup restore verified; data reconciles |

**Order:** O1->O2->O3->O4->O5->O6->O7->O8->O9->O10->O11->O12->O13->O14
**Acceptance:** Load/stress/soak/chaos tests run; raw results recorded; no invented metrics; SLOs verified


## Global acceptance gates (validate before VERIFIED)
- Rust: cargo fmt --all -- --check | cargo clippy --workspace --all-targets -- -D warnings | cargo check --workspace --all-targets | cargo test --workspace --all-targets | cargo test --workspace --doc | cargo build --workspace --all-targets
- Go: cd smtp-api && gofmt -l . && go test ./... -race && go vet ./... && go build ./...
- Dashboard: cd smtp-dashboard && pnpm install --frozen-lockfile && pnpm lint:dashboard && pnpm typecheck:dashboard && pnpm build:dashboard
- Infra: docker compose config --quiet | ./scripts/validate-migrations.sh | ./scripts/deploy-preflight.sh --production-env
- DB: integration tests against disposable Postgres (TEST_DATABASE_URL)
- Security: cargo audit; dependency scanning; SBOM; secret scanning; fuzz results

## Implementation sequence (recommended)
1. A1-A13 Security & Identity (hard dependency for all later milestones)
2. B1-B10 Control Plane & Multi-Tenancy (PostgreSQL state)
3. C1-C12 SMTP Protocol & Ingress (protocol conformance)
4. D1-D14 Durable Queue & Outbound Delivery (queue + delivery)
5. E1-E13 Inbound Mail & Deliverability (inbound routing + scanning + auth)
6. F1-F12 DKIM, Domain Security & DNS (DKIM lifecycle + DNS checks)
7. G1-G12 Mailbox & IMAP (mailbox protocols)
8. H1 POP3 scope decision (exclude or implement)
9. I1-I14 Object Storage & Attachments (storage backend)
10. J1-J11 Bounces, Suppressions & Events (bounce handling + webhooks)
11. K1-K15 API & Dashboard Integration (versioned APIs)
12. L1-L11 Search, Retention & Data Lifecycle
13. M1-M16 Observability & Production Operations
14. N1-N16 Security & Supply Chain
15. O1-O14 Performance & Scale Validation

## Notes on current codebase state
- Already partially implemented: A4, C1-C11, D1-D3, D5-D12, E4-E7, F3/F8/F9 (partial), G1-G11 (partial), J1-J10 (partial), K1 (partial), M16 (partial), C9 BROKEN, A10 BROKEN, N1 done.
- MISSING: B1-B10 (control plane state), F1-F12 (DKIM full), G12 (IMAP), I1-I14 (storage), J11 (idempotency), K1-K15 (full API), L1-L11 (search/retention), M1-M15 (full observability), N2-N16 (supply chain), O1-O14 (perf/scroll).
- BLOCKED-EXTERNAL: production DNS, provider secret-manager APIs, cloud deployment target, physical load/HW, provider-specific service configuration.
- Deliberate scope decisions: POP3 excluded from first production release; IMAP kept first-class but bounded; outbound delivery worker disabled by default until tenant configuration supplied.
