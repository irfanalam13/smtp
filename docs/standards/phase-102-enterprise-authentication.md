# Phase 102 — Enterprise Authentication, SSO, and MFA

Status: **Keycloak OIDC access-token verification implemented; browser SSO/session lifecycle, MFA policy, and SAML remain pending**

## Repository baseline

The API can optionally validate Keycloak OIDC bearer access tokens using a
configured issuer and API audience. It maps `(issuer, subject)` to a
pre-provisioned active user and tenant membership in PostgreSQL; email claims
do not grant or link access. A user with multiple active memberships must
select one using `X-Tenant-ID`, which is checked against the database. The
development identity adapter is explicitly development-only. API keys remain
available for machine access.

There is no browser authorization-code/PKCE flow, session cookie lifecycle,
MFA policy enforcement in the application, SAML service provider, account
recovery, SCIM, or user-provisioning workflow.

## Recommended design

Use an established identity provider rather than implementing an identity
provider or password store. Prefer OIDC for new integrations; add SAML only
where customer requirements demand it. Validate issuer, audience, signature,
expiry, nonce/state, redirect URI, key rotation, and organization/domain
binding. Map external identities to durable users and tenants through an
explicit verified provisioning flow. Enforce phishing-resistant MFA for
privileged/operator accounts and define customer policy capabilities.

Account recovery, IdP unlinking, invitation acceptance, session revocation,
SCIM lifecycle (if required), and emergency access are security-critical
flows and need independent tests. Do not equate an email domain claim with
tenant authorization.

## Remaining work and why

Complete Keycloak realm/client configuration, operator-controlled user
provisioning and deprovisioning, OIDC authentication integration tests against
an isolated PostgreSQL database, and a browser authorization-code/PKCE or
backend-for-frontend flow with secure session/CSRF/logout/revocation controls.
Set and test Keycloak MFA policy for privileged accounts. Determine whether
SAML/SCIM is required and perform interoperability/account-takeover testing.
This remains open because the resource-server verifier does not provide an
interactive user lifecycle or prove the realm's MFA/configuration controls.

## Acceptance criteria

- Protocol validation covers issuer/audience/signature/state/nonce/expiry and
  key rotation; invalid assertions are rejected.
- Account linking and tenant membership require verified, auditable
  authorization and cannot be inferred solely from an email suffix.
- Privileged accounts enforce approved MFA; recovery and revocation are
  tested.
- Sessions use secure cookie attributes, bounded lifetime, rotation, and
  server-side revocation semantics.
- SAML is implemented only against a documented profile and interoperability
  suite when required.
- API bearer-token verification checks signature, exact issuer, API audience,
  expiry, Keycloak bearer-token type, bounded token size, and a provisioned
  active database membership.

## Research references

- [Keycloak securing applications and OIDC layers](https://www.keycloak.org/securing-apps/oidc-layers)
- [OpenID Connect Core 1.0](https://openid.net/specs/openid-connect-core-1_0.html)
- [NIST SP 800-63B-4 Digital Identity Guidelines](https://pages.nist.gov/800-63-4/sp800-63b.html)
- [Phase 74 Authentication and Identity](phase-74-authentication-identity.md)
