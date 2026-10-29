# Phase 14 — SMTP Security

Status: **Secure transport foundation implemented; STARTTLS upgrade remains
pending**

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
- `AUTH` is refused with `538` on a plaintext session when
  `require_tls_for_auth` is enabled (the default).
- The existing AUTH verifier never receives credentials unless the transport
  is marked as protected.
- TLS configuration is optional for local test mode and required when
  `implicit_tls` is enabled.

## Not falsely advertised

`STARTTLS` is still not advertised or accepted. Supporting it correctly
requires upgrading the existing connection in-place, reassembling the live
stream after the `220` response, resetting the SMTP transaction, and ensuring
no pipelined plaintext is interpreted as TLS bytes. That refactor is the next
security milestone; returning `502` is safer than accepting a command without
performing encryption.

The server also does not yet implement client-certificate authentication,
certificate rotation, OCSP/stapling policy, cipher-suite reporting, or
production secret-manager integration.

## Acceptance boundary

The secure foundation is complete when certificate loading rejects missing or
invalid material, implicit TLS performs a real Rustls handshake, plaintext
AUTH is rejected, and TLS handshake failures do not downgrade. STARTTLS
requires separate end-to-end tests for state reset and post-upgrade AUTH.
