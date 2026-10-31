# Cryptography and TLS policy

The platform uses established standard-library and protocol libraries. It does
not implement cryptographic algorithms, certificate validation, or a CA.

Current controls:

- `crypto/rand` generates identifiers and API-key secrets.
- API-key verification uses SHA-256 plus constant-time comparison.
- Browser-session secrets and CSRF tokens use `crypto/rand`; only SHA-256
  verifiers are persisted. Password hashing primitives in `smtp-auth` use
  bounded Argon2id parameters but are not yet wired to an account flow.
- OIDC verification uses the maintained `go-oidc` library and provider
  discovery/JWKS rather than custom signature code. Login uses Authorization
  Code with PKCE; state, nonce, issuer, audience, signature, expiration, and
  issued-at are validated.
- Outbound SMTP uses Rustls with the system/web PKI roots and validates the MX
  hostname during STARTTLS.
- Advertised STARTTLS failures do not silently downgrade.
- Ingress SMTP does not advertise TLS or AUTH until a reviewed listener
  configuration exists.

API-key hashing is suitable for high-entropy random tokens, not passwords.
Email/password authentication, JWT refresh-token lifecycle, MFA, and passkeys
are not implemented. Do not treat the Argon2id helper as a complete account
authentication system.

Production certificate issuance, private-key storage, rotation, KMS/HSM
integration, and internal mTLS remain deployment requirements.
