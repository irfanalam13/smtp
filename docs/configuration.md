# Configuration reference

Configuration is supplied through environment variables. `.env` is for local
development only and must not be committed or reused as a production secret
file.

## Local configuration

| Variable                             | Required              | Default                     | Purpose                                                                                                               |
| ------------------------------------ | --------------------- | --------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `POSTGRES_PASSWORD`                  | Yes                   | None                        | Compose development database password                                                                                 |
| `DATABASE_URL`                       | Queue/integration use | Derived locally             | PostgreSQL connection URL                                                                                             |
| `API_ADDR`                           | No                    | `127.0.0.1:8080`            | Go API listen address; the local runner rejects non-loopback addresses                                                |
| `API_URL`                            | No                    | `http://127.0.0.1:8080`     | Backend origin used by the Next.js dashboard rewrite                                                                  |
| `APP_ENVIRONMENT`                    | No                    | Unset                       | Environment name; `development`/`dev` permits loopback HTTP auth cookies, all other environments require HTTPS         |
| `APP_PUBLIC_ORIGIN`                  | Browser sessions      | Development loopback        | Canonical dashboard origin used for OIDC callbacks and same-origin checks; required outside development               |
| `CONTROL_PLANE_DEV_MODE`             | No                    | `false`                     | Enables the local `X-Dev-Subject` principal adapter                                                                   |
| `OIDC_ISSUER_URL`                    | Optional              | Unset                       | OIDC issuer for API access-token verification and browser login                                                       |
| `OIDC_API_AUDIENCE`                  | API OIDC              | Unset                       | Expected audience for bearer access tokens; the browser ID token is not an API token                                  |
| `OIDC_CLIENT_ID`                     | Browser OIDC          | Unset                       | Server-side OIDC authorization-code client ID; configure with issuer and secret                                        |
| `OIDC_CLIENT_SECRET`                 | Browser OIDC          | Unset                       | Server-side provider client secret; keep in a secret manager, never expose to browser code                             |
| `OIDC_REDIRECT_URI`                  | Browser OIDC          | Derived from public origin | Must exactly equal `<APP_PUBLIC_ORIGIN>/api/backend/v1/auth/oidc/callback`                                             |
| `AUTH_SESSION_IDLE_TTL`              | No                    | `30m`                       | Sliding browser-session idle lifetime; minimum 1 minute                                                               |
| `AUTH_SESSION_ABSOLUTE_TTL`          | No                    | `12h`                       | Maximum browser-session lifetime; must be at least idle TTL and at most 30 days                                        |
| `AUTH_SESSION_COOKIE_NAME`           | No                    | Environment dependent       | Session cookie name; secure origins require the `__Host-` prefix                                                      |
| `AUTH_CSRF_COOKIE_NAME`              | No                    | Environment dependent       | Readable double-submit CSRF cookie name; secure origins require the `__Host-` prefix                                 |
| `NEXT_PUBLIC_AUTH_CSRF_COOKIE_NAME`  | Custom CSRF cookie    | Unset                       | Dashboard-visible CSRF cookie name; must match `AUTH_CSRF_COOKIE_NAME` when customized                                |
| `AUTH_SMTP_ADDR`                     | Password account email | Unset                      | Authentication email relay as `host:port`; port 465 uses implicit TLS and other ports require STARTTLS               |
| `AUTH_EMAIL_FROM`                    | Password account email | Unset                      | Valid From mailbox for email verification and password-recovery messages                                               |
| `AUTH_SMTP_USERNAME`                 | SMTP relay auth       | Unset                       | Optional SMTP relay username; must be configured together with `AUTH_SMTP_PASSWORD`                                    |
| `AUTH_SMTP_PASSWORD`                 | SMTP relay auth       | Unset                       | Optional SMTP relay password; keep it in a secret manager                                                              |
| `MAILBOX_SERVICE_ADDR`               | No                    | `127.0.0.1:8081`            | Loopback listener for the Rust PostgreSQL mailbox service                                                             |
| `MAILBOX_SERVICE_URL`                | No                    | `http://127.0.0.1:8081`     | Go API's mailbox-service origin; non-loopback origins require HTTPS                                                   |
| `MAILBOX_SERVICE_HMAC_KEY`           | Mailbox API           | None                        | Shared secret of at least 32 bytes encoded as hexadecimal; signs short-lived internal principal claims                |
| `SMTP_DELIVERY_ENABLED`              | No                    | `false`                     | Starts outbound delivery when explicitly enabled                                                                       |
| `SMTP_RELAY_HOST`                    | Optional relay delivery | None                      | Authenticated relay hostname; leave every `SMTP_RELAY_*` setting unset for direct-MX delivery                          |
| `SMTP_RELAY_PORT`                    | SMTP relay delivery   | None                        | Required relay port: `587` (STARTTLS) or `465` (implicit TLS)                                                           |
| `SMTP_RELAY_USERNAME`                | Optional relay delivery | None                      | Relay account username; keep it in a secret manager                                                                     |
| `SMTP_RELAY_PASSWORD`                | Optional relay delivery | None                      | Relay account password; keep it in a secret manager                                                                     |
| `SMTP_RELAY_FROM`                    | Optional relay delivery | None                      | Legacy relay configuration address; tenant authorization still applies                                                  |
| `SMTP_DELIVERY_ALLOWLIST`            | Direct-MX delivery    | None                        | Comma-separated exact destination domains permitted when no authenticated relay is configured                         |
| `SMTP_DELIVERY_TLS_POLICY`           | No                    | `opportunistic`             | Outbound STARTTLS policy: `opportunistic` or `required`                                                               |
| `SMTP_DELIVERY_MAX_CONCURRENT`       | No                    | `16`                        | Maximum number of concurrent per-domain SMTP delivery batches per worker                                              |
| `SMTP_DELIVERY_MAX_MESSAGE_BYTES`    | No                    | `10485760`                  | Outbound message limit; cannot exceed the canonical parser's 10 MiB ceiling                                           |
| `SMTP_DELIVERY_EHLO_HOSTNAME`        | No                    | `smtp-platform.local`       | Hostname advertised by the outbound client                                                                            |
| `SMTP_DKIM_KEY_DIR`                  | Direct-MX delivery    | None                        | Secret-manager projected key root; direct-MX delivery fails startup without it                                        |
| `SMTP_DKIM_REPORT_DOMAIN`            | DMARC report worker   | None                        | Receiver domain used in report metadata, report IDs, and attachment filenames                                         |
| `SMTP_DKIM_REPORT_SENDER`            | DMARC report worker   | None                        | RFC 5322 From mailbox; its domain must equal `SMTP_DKIM_REPORT_DOMAIN` and have an active DKIM key                    |
| `SMTP_DKIM_REPORT_ORG_NAME`          | DMARC report worker   | None                        | Reporting organization name placed in aggregate report metadata                                                       |
| `SMTP_DKIM_REPORT_CONTACT`           | DMARC report worker   | None                        | Contact mailbox placed in aggregate report metadata                                                                   |
| `SMTP_BIND_ADDR`                     | No                    | `127.0.0.1:2525`            | SMTP ingress listener address; non-loopback submission requires authentication or an inbound recipient-policy adapter |
| `SMTP_LISTENER_PROFILE`              | No                    | `loopback-development`      | Explicit role: `loopback-development`, `inbound-mta`, or authenticated `submission`; role combinations are validated at startup |
| `SMTP_MAX_MESSAGE_BYTES`             | No                    | `10485760`                  | Global SMTP message-size ceiling advertised in EHLO and enforced during DATA; maximum 10 MiB                          |
| `SMTP_INBOUND_ENABLED`               | No                    | `false`                     | Wires verified-local-recipient PostgreSQL acceptance into SMTP; requires `DATABASE_URL`                               |
| `SMTP_INBOUND_MAX_MESSAGE_BYTES`     | No                    | `SMTP_MAX_MESSAGE_BYTES`    | Optional lower global ceiling for inbound messages; recipient tenant/domain limits can lower it further               |
| `SMTP_INBOUND_RATE_LIMIT_HMAC_KEY`   | Public inbound        | None                        | Shared secret of at least 32 bytes used to hash IP/domain keys for PostgreSQL inbound rate limits                     |
| `SMTP_INBOUND_MAX_MESSAGES_PER_HOUR` | No                    | `10000`                     | Shared per-IP and per-domain message attempt limit; applies to inbound SMTP DATA transactions                         |
| `SMTP_CLAMD_ADDR`                    | Public inbound        | None                        | ClamAV `clamd` IP:port endpoint; messages fail temporarily if the configured scanner is unavailable                   |
| `SMTP_CLAMD_TIMEOUT_SECONDS`         | No                    | `10`                        | Whole-message clamd scan timeout, bounded from 1 to 60 seconds                                                        |
| `SMTP_SPAMD_ADDR`                    | Public inbound        | None                        | SpamAssassin `spamd` IP:port endpoint; flagged spam is rejected and scanner outages fail temporarily                  |
| `SMTP_SPAMD_TIMEOUT_SECONDS`         | No                    | `10`                        | Whole-message spamd scan timeout, bounded from 1 to 60 seconds                                                        |
| `SMTP_ENABLE_8BITMIME`               | No                    | `false`                     | Enables and advertises the implemented 8BITMIME body profile                                                          |
| `SMTP_TLS_CERT` / `SMTP_TLS_KEY`     | TLS use               | None                        | PEM certificate chain and private key; configure both to enable STARTTLS                                              |
| `SMTP_IMPLICIT_TLS`                  | No                    | `false`                     | Starts SMTP with TLS already negotiated; use a dedicated implicit-TLS submission listener/profile                     |
| `SMTP_AUTH_ENABLED`                  | No                    | `false`                     | Enables PostgreSQL-backed SMTP AUTH PLAIN; requires `DATABASE_URL` and TLS certificate/key                            |
| `SMTP_RATE_LIMIT_HMAC_KEY`           | SMTP AUTH             | None                        | Shared secret (at least 32 bytes) used to pseudonymize distributed PostgreSQL auth-throttle keys                      |
| `SMTP_REQUIRE_AUTHENTICATION`        | No                    | Mirrors `SMTP_AUTH_ENABLED` | Requires authentication before accepting `MAIL FROM`; disabling it while AUTH is enabled is rejected                  |
| `AUTH_EMAIL_ENCRYPTION_KEY`          | Account email delivery | None                     | At least 32 random bytes used to encrypt verification/reset notification bodies in the PostgreSQL outbox              |
| `TEST_DATABASE_URL`                  | Queue tests           | None                        | Isolated disposable PostgreSQL database for ignored integration tests                                                 |

### Dashboard API proxy

The Next.js dashboard sends browser API requests to the same-origin
`/api/backend/*` rewrite. `API_URL` is read by the Next.js server and must be
an absolute HTTP(S) origin without credentials, path, query, or fragment.
Development defaults to `http://127.0.0.1:8080`; set `API_URL` explicitly for
deployed builds and use HTTPS in production. Do not rename it to a
`NEXT_PUBLIC_` variable: exposing the upstream origin is unnecessary, and the
browser does not call it directly. This same-origin design avoids the need for
browser CORS access to the API.

### Browser authentication

The API enables browser sessions when PostgreSQL and a valid `APP_PUBLIC_ORIGIN`
are configured. OIDC sign-in additionally requires `OIDC_ISSUER_URL`,
`OIDC_CLIENT_ID`, and `OIDC_CLIENT_SECRET`. Register the exact redirect URI
`<APP_PUBLIC_ORIGIN>/api/backend/v1/auth/oidc/callback` with the provider. The
optional `OIDC_API_AUDIENCE` remains dedicated to API bearer access tokens.
Production and non-development environments require an HTTPS public origin,
Secure `__Host-` cookies, and server-side secret storage. Development defaults
to `http://127.0.0.1:3210`; HTTP is allowed only for a loopback host.

Password signup, email verification, and password recovery require
`AUTH_SMTP_ADDR` and `AUTH_EMAIL_FROM`; SMTP authentication credentials are
optional but must be configured as a pair. The API enforces implicit TLS on
port 465 and STARTTLS on other ports. Without a configured relay, password
login can remain available for existing accounts, while signup and recovery
are reported as unavailable by `GET /v1/auth/providers`.

The transactional email API selects its transport only from server configuration.
For production direct-MX, set `SMTP_DELIVERY_ENABLED=true`, leave every
`SMTP_RELAY_*` setting unset, set an exact controlled-test recipient allowlist,
and mount tenant-domain DKIM keys at `SMTP_DKIM_KEY_DIR`. The API accepts a From
address only after the authenticated tenant has completed its DNS ownership
challenge for that exact domain. It persists the tenant, sender domain,
`direct_mx` mode, and a non-secret `secret-manager://dkim/<domain>` reference;
the worker loads the corresponding mounted key and never falls back to a relay.

For local development, use the exact development-only Mailpit endpoint
`127.0.0.1:1025` with empty relay credentials. The API marks these messages as
`mailpit`. An authenticated TLS relay remains available for compatibility, but
is a separate `relay` mode. The composer exposes To, Cc, and Bcc; Bcc is stored
only as envelope-recipient metadata. SMTP acceptance is not inbox placement.
Apply queue migration `000032` before enabling tenant direct-MX submissions.

Apply migrations `000028` and `000029` before enabling browser sessions and
password accounts. Signup creates a new workspace with the registering user as
owner; the address must be verified before password login. Password reset links
are single-use and expire after 30 minutes; changing a password revokes the
user's existing browser sessions. The dashboard exposes `/signup`, `/login`,
`/forgot-password`, `/reset-password`, and `/verify-email`; dashboard feature
routes use paths such as `/transaction/email/send`,
`/deliverability/domains`, `/inbound/mailboxes`, and `/developers/api-keys`.
Sessions resolve OIDC identities by exact issuer and subject and password
identities by the stored account, then require an active tenant membership.
Rate limiting for OIDC and session exchange is currently process-local; use
an edge limit and do not treat it as distributed abuse protection.

## Safety rules

- Keep `API_ADDR`, PostgreSQL, and SMTP loopback-bound during development.
- Inbound SMTP is disabled by default. Enable it only with migrated PostgreSQL
  storage; inbound fan-out is atomic and restricted to mailboxes owned by one
  tenant per SMTP transaction. Non-loopback inbound requires clamd and spamd
  scanners plus the shared inbound rate-limit HMAC key. Scanner outages fail
  closed with a temporary SMTP error; malware and spam policy matches receive
  permanent rejection. Inbound SPF/DKIM/DMARC checks use the bounded resolver
  and standards implementation; DMARC reject policy is rejected and quarantine
  policy is stored separately with an outbox event.
- Do not set `CONTROL_PLANE_DEV_MODE=true` in deployment.
- Do not enable direct-MX delivery without a reviewed destination policy.
- Direct-MX delivery fails closed unless an explicit recipient-domain
  allowlist is configured. Entries match exact normalized DNS domains; no
  wildcards or suffix matching are supported. Authenticated relay delivery
  instead sends through the configured SMTP relay and requires no direct-MX
  allowlist.
- SMTP AUTH PLAIN is accepted only after STARTTLS or implicit TLS. Never expose
  authenticated submission without valid certificate material.
- Keep `SMTP_CLAMD_ADDR` reachable only over a trusted private network; clamd
  traffic is plaintext and the endpoint must not be supplied by request data.
- For outbound signing, mount the M07 Ed25519 PKCS#8 DER keys from a secret
  manager under `SMTP_DKIM_KEY_DIR/<domain>/<selector>.pk8`; key files must have
  no group or world permissions. The `active-selector` file names the selector
  used for new mail. Publish its DNS TXT record and wait for propagation before
  switching that file; retain the previous DNS key through the overlap window.
  Revocation publishes an empty `p=` record before retiring the key. Never put
  DKIM private keys in PostgreSQL or logs.
  Generate a selector key with:

  ```sh
  cargo run -p mail-delivery --bin dkim-keygen -- example.com 2026q4 /run/secrets/dkim
  ```

  Publish the printed TXT record, wait for DNS propagation, then run
  `cargo run -p mail-delivery --bin dkim-keyctl -- activate example.com 2026q4 /run/secrets/dkim`.
  Activation checks the DNSSEC-validated TXT key matches the private key and
  atomically switches `active-selector`. For rotation, keep the old public DNS
  key published for at least its TTL after switching. Remove the old public
  record, wait through its TTL, then run
  `cargo run -p mail-delivery --bin dkim-keyctl -- retire example.com old-selector /run/secrets/dkim --dns-removed-after-ttl`.
  To revoke a compromised selector, remove its DNS public key immediately and
  activate a clean selector before retiring its private file. The retire flag
  records the operator's confirmation that DNS removal and TTL expiry are done.

- Apply migration `000025` before running DMARC reporting. Run
  `cargo run -p mail-delivery --bin dmarc-report-worker` alongside the regular
  delivery worker. It produces one UTC-day report per policy domain and
  authorized `rua` destination, stores immutable report batches, then enqueues
  them through the normal outbound pipeline. Add the exact authorized report
  recipient domains to `SMTP_DELIVERY_ALLOWLIST`, keep a DKIM key mounted for
  `SMTP_DKIM_REPORT_DOMAIN`, and use `SMTP_DELIVERY_TLS_POLICY=required` when
  report transport must require STARTTLS. Report data is retained for 90 days.
- `dmarc-report-ingest <report.xml[.gz]|message.eml>` validates and deduplicates
  an external RFC 9990 report, then stores its normalized records for 90 days.
  For `.eml`, it extracts bounded XML/gzip MIME attachments. Feed it only
  messages that have passed the normal inbound authentication and mailbox
  access policy.
- Provide the same `SMTP_INBOUND_RATE_LIMIT_HMAC_KEY` to all inbound replicas.
  Rotating it resets the effective IP/domain rate buckets and should be a
  coordinated deployment.
- When `SMTP_AUTH_ENABLED=true`, provide the same `SMTP_RATE_LIMIT_HMAC_KEY`
  to every ingress instance through the deployment secret provider. Shared
  PostgreSQL counters enforce IP, tenant, and account lockouts. Rotate this
  key only as a coordinated deployment because changing it resets the
  effective throttle buckets.
- SMTP TLS certificate and key files are checked for changes every 30 seconds.
  Valid replacement material is activated for new connections; invalid or
  mismatched files leave the last valid certificate active and emit an error.
- PostgreSQL tenant message-size ceilings default to 10 MiB and may be stored
  as high as 1 GiB for future deployment profiles. The current SMTP, queue,
  and MIME-parser ceilings remain 10 MiB, so a higher tenant setting never
  raises the effective limit. A verified domain may set an optional lower
  ceiling. Authenticated submissions use the minimum of global, tenant, and
  verified sender-domain limits. Operators can configure the
  durable limits with `UPDATE tenants SET max_smtp_message_bytes = ... WHERE id = ...`
  and `UPDATE domains SET max_smtp_message_bytes = ... WHERE id = ...`;
  domain `NULL` inherits the tenant limit.
- Mount certificate/key and environment secrets from the deployment's secret
  manager (for example via a secrets CSI driver); the application does not
  fetch secrets directly from a cloud provider API.
- Do not point `TEST_DATABASE_URL` at a database containing useful data.
- Store production secrets in a secret manager, not `.env`.
- Configure both OIDC variables or neither. Production issuer URLs must use
  HTTPS; HTTP is accepted only for loopback development issuers.
- Use a dedicated API audience in Keycloak access tokens; do not use an ID
  token as an API bearer token.

## Configuration ownership

The Go API currently reads `API_ADDR`, `CONTROL_PLANE_DEV_MODE`, and the
optional Keycloak OIDC issuer/audience pair.
The SMTP ingress and queue read `DATABASE_URL` through their Rust
configuration. SMTP AUTH is opt-in and shares that database connection
configuration. SMTP credentials are created and managed through the
authenticated Go control-plane API; the SMTP AUTH rate-limit key must be
identical across ingress replicas. The external delivery worker reads queue
storage settings through `mail-queue` and transport settings through
`mail-delivery`. The development scripts export shared values before starting
processes.
