# Phase 14 — SMTP Security

Status: **Ingress STARTTLS and authenticated submission implemented**

## Research basis

- [RFC 3207](https://www.rfc-editor.org/rfc/rfc3207) defines the SMTP
  STARTTLS extension and requires the SMTP transaction state to be reset after
  TLS negotiation.
- [RFC 4954](https://www.rfc-editor.org/rfc/rfc4954) requires AUTH to be
  protected by an appropriate security layer when credentials are used.
- [RFC 8314](https://www.rfc-editor.org/rfc/rfc8314) recommends TLS for
  submission and deprecates cleartext credential transport.

## Implemented

- Rustls server configuration from an X.509 certificate chain and private-key
  PEM files.
- Optional implicit TLS at connection acceptance using `TlsAcceptor`.
- TLS failures are explicit connection errors; there is no plaintext fallback
  after a TLS handshake failure.
- STARTTLS is advertised only when TLS material is configured and upgrades the
  existing connection with a bounded handshake timeout.
- Pipelined/buffered plaintext is rejected at the STARTTLS boundary. After a
  successful handshake, EHLO and transaction state are reset.
- AUTH PLAIN is advertised only after TLS and plaintext AUTH is rejected
  before a challenge or credential-verifier call.
- The PostgreSQL verifier scopes credential lookups to an active tenant,
  stores versioned Argon2id hashes, and enforces expiry and revocation.
- Authenticated `MAIL FROM` is accepted only for a verified domain owned by
  the authenticated tenant; missing policy configuration fails closed.
- Per-IP, per-tenant, and per-account authentication throttles, bounded
  connections/commands, and graceful shutdown are implemented. Auth throttle
  state is persisted in PostgreSQL and shared across ingress processes; bucket
  identities are HMACed with a deployment-provided key.
- SMTP credential list/create/rotate/revoke operations are available through
  the tenant-scoped control-plane API. Generated passwords are shown once;
  Argon2id hashes use the same parameters as SMTP verification, and lifecycle
  events are audited transactionally.
- Certificate/key files are polled every 30 seconds. A complete valid pair
  replaces the active Rustls configuration atomically; failures retain the
  last valid configuration.
- TLS configuration is optional for local test mode and required when
  `implicit_tls` is enabled.

Production configuration must provide the same `SMTP_RATE_LIMIT_HMAC_KEY` to
all ingress replicas. The app supports manager-backed mounted secret files and
environment injection; it does not call a specific cloud secret-manager API.
Python's standard `smtplib` completes STARTTLS, AUTH PLAIN, and a submission
transaction in the database-backed integration suite. A wider client matrix,
client-certificate authentication, OCSP/stapling policy, and outbound
delivery-worker integration remain unverified or unimplemented.

## Acceptance boundary

The network suite verifies STARTTLS state reset, AUTH gating, a complete
authenticated SMTP transaction, pipelined plaintext rejection, handshake
failure without downgrade, connection limits, slow-client deadlines, and
graceful shutdown. Unit tests verify TLS file rotation preserves the previous
configuration on invalid replacement material. PostgreSQL integration tests
verify shared lockout across instances, stored hashes, tenant isolation,
expiry, rotation, and revocation when run against a disposable migrated
PostgreSQL database, including verified-domain sender authorization.
