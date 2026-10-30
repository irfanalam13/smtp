# Phase 74 — Authentication and Identity

Status: **Durable API-key foundation exists; production user identity and sessions pending**

## Repository baseline

The Go API supports a development-only subject header, durable PostgreSQL API
keys, and optional Keycloak OIDC bearer-token verification for provisioned
users. Keys are high-entropy, hashed, scoped, expirable, revocable, and
user-bound for mailbox access. The OIDC resource-server verifier validates
the configured issuer, API audience, signature, expiry, and bounded token
size, then resolves the issuer/subject against active PostgreSQL membership.
Browser login/session lifecycle, MFA policy enforcement, SAML, device
management, and automated identity provisioning remain unimplemented. The
accepted authentication boundary explicitly rejects a custom password/token
issuer.

## Recommended identity architecture

Use a standards-compliant external OpenID Connect provider for interactive
identity; validate issuer, audience, signature, algorithm, expiry, nonce/state,
and redirect URI according to the provider and current OAuth security best
practice. Map identities by `(issuer, subject)`, not email address. Treat
identity claims as authentication input only; load current account status,
tenant membership, and authorization from authoritative platform records.

Use authorization-code flow with PKCE for browser/public clients. Keep browser
tokens out of persistent local storage; use secure, HttpOnly, SameSite cookies
or a backend-for-frontend pattern with CSRF defenses, short session lifetimes,
rotation, revocation, and reauthentication for sensitive actions. Delegate
password policy, MFA, SAML federation, and enterprise SSO to the selected
identity provider unless product requirements justify a documented exception.

Keep API keys for machine access, with narrow scopes, owner/service-account
binding, expiry, rotation, one-time secret display, and audit. Service accounts
must be distinct principals with explicit tenant ownership; never treat an
API key as a user session. Provisioning/deprovisioning interfaces such as
SCIM are an optional follow-on after the IdP contract is selected.

## Remaining work and why

Configure and test the selected Keycloak realm/client; define issuer, API
audience, claims, account linking, invite/first-login behavior, session
architecture, MFA policy,
service-account flows, revocation propagation, recovery, and audit. Implement
the OIDC verifier and browser lifecycle, then prove invalid issuer/audience,
expired/replayed tokens, disabled users, removed memberships, and key revocation
fail closed.

This remains open because no issuer or audience is configured, and the current
development identity must never become production authentication. MFA/SAML
are provider and enterprise policy choices; implementing them independently
would add a high-risk identity system without a selected trust model.

## Acceptance criteria

- Production rejects development headers and unsigned/untrusted identities.
- User identity mapping is stable across email changes and prevents account
  linking by unverified email.
- Issuer, audience, signature, expiry, nonce/state, and redirect handling are
  validated; session cookies and CSRF controls are tested.
- Disabled users, removed memberships, expired/revoked API keys, and disabled
  service accounts lose access within a documented bound.
- Secrets, bearer tokens, auth codes, and session identifiers are not logged.

## Research references

- [OpenID Connect Core 1.0, incorporating errata set 2](https://openid.net/specs/openid-connect-core-1_0.html)
- [RFC 9700 — OAuth 2.0 Security Best Current Practice](https://www.rfc-editor.org/rfc/rfc9700.html)
- [API-key handling ADR](../architecture/adr/0007-api-key-handling.md)
- [Control-plane authentication ADR](../architecture/adr/0006-control-plane-auth-and-isolation.md)
