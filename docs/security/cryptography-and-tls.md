# Cryptography and TLS policy

The platform uses established standard-library and protocol libraries. It does
not implement cryptographic algorithms, certificate validation, or a CA.

Current controls:

- `crypto/rand` generates identifiers and API-key secrets.
- API-key verification uses SHA-256 plus constant-time comparison.
- Outbound SMTP uses Rustls with the system/web PKI roots and validates the MX
  hostname during STARTTLS.
- Advertised STARTTLS failures do not silently downgrade.
- Ingress SMTP does not advertise TLS or AUTH until a reviewed listener
  configuration exists.

API-key hashing is suitable for high-entropy random tokens, not passwords.
Password authentication is not part of the approved current identity model and
must not be added without an identity decision and Argon2id lifecycle design.

Production certificate issuance, private-key storage, rotation, KMS/HSM
integration, and internal mTLS remain deployment requirements.
