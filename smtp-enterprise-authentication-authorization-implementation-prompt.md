# SMTP Platform — Enterprise Authentication & Authorization Implementation Prompt

## Mission

Implement a complete, production-grade authentication and authorization system for the SMTP Email Cloud Platform. Treat this as a security-critical platform foundation, not a UI-only login feature.

Act as a principal identity/security architect and senior full-stack engineer. Inspect the entire repository before changing code. Build on the existing architecture and supported backend contracts; do not introduce a parallel identity system or casually replace existing infrastructure.

The implementation must support secure first-party accounts, enterprise identity providers, strong multi-factor authentication, session/device management, tenant isolation, granular authorization, administrative controls, auditability, and secure recovery flows.

**Do not stop after planning, scaffolding, mock screens, or a basic OAuth callback. Implement, test, document, and validate the complete scope that is supportable by the repository.**

---

## 1. Mandatory repository discovery

Before implementation:

1. Inspect the repository structure, workspace manifests, dependency versions, environment configuration, deployment files, migrations, API routes, middleware, frontend routes, existing identity code, and security documentation.
2. Read all relevant architecture/API/OpenAPI documents and existing authentication-related code.
3. Identify the actual backend and frontend boundaries, current user/principal/tenant models, API-key behavior, OIDC integration (including Keycloak if present), session model, token validation, and authorization checks.
4. Identify what is implemented, partially implemented, simulated, or absent.
5. Write a concise implementation plan and threat model in repository documentation.
6. Preserve existing working behavior and compatibility where safe. Do not silently break API-key authentication, service-to-service access, or existing tenant ownership rules.
7. Do not assume an endpoint, database table, environment variable, or provider integration exists until verified in source.

Use existing language, frameworks, migrations, conventions, test tools, and deployment patterns. Add dependencies only when justified and maintained.

---

## 2. Identity and authentication architecture

Design one coherent identity architecture with a clear separation between:

- **Authentication:** proving who or what is making a request.
- **Authorization:** determining what that identity may do.
- **Identity federation:** trusting external identity providers.
- **Sessions:** maintaining and revoking authenticated browser/device state.
- **Tenant context:** selecting and enforcing the organization/workspace boundary.
- **Service identity:** authenticating workloads, API clients, and SMTP infrastructure.

Create explicit domain models and interfaces. Avoid coupling business logic directly to one provider.

Support these identity categories:

1. Human users.
2. Organization/tenant members.
3. Tenant administrators and platform administrators.
4. Service accounts and machine identities.
5. API-key clients.
6. SMTP submission clients and mailbox users, where the product's protocol design requires them.

Use immutable internal user IDs as the platform identity. External provider subject IDs must be stored as provider-scoped identities and must never be treated as globally unique on their own.

Define account linking and identity resolution safely. Never automatically merge accounts solely because an OAuth provider returns a matching email address. Require a verified, authenticated linking flow and explicit user confirmation or a controlled enterprise provisioning policy.

---

## 3. Authentication methods

Implement methods in a modular, extensible way. Only expose a provider as enabled when its credentials, callback URLs, configuration, and validation are complete.

### 3.1 First-party email and password

Implement:

- Registration, if enabled by product policy.
- Login and logout.
- Email verification with single-use, expiring, high-entropy tokens.
- Resend verification with rate limits and abuse controls.
- Password change for authenticated users.
- Password reset with single-use, expiring tokens.
- Secure password policy based on current security guidance; avoid arbitrary complexity rules that encourage weak passwords.
- Password breach screening where a reliable, privacy-preserving integration is available.
- Password hashing using Argon2id with reviewed parameters, per-password salts, and safe migration support from any existing hash format.
- Generic responses that do not disclose whether an email address has an account.
- Protection against brute force, credential stuffing, enumeration, and timing differences.
- Re-authentication for sensitive account changes.

Never store plaintext passwords, reversible password encryption, reset tokens, verification tokens, or raw MFA secrets.

### 3.2 Google sign-in

Implement Google OpenID Connect using a maintained, standards-compliant library.

Validate:

- Issuer.
- Audience/client ID.
- Signature and signing-key rotation.
- Expiration and issued-at time.
- Nonce.
- State.
- Redirect URI.
- PKCE where applicable.
- Required scopes and verified email claim.

Use Authorization Code flow. Do not use implicit flow or trust client-submitted profile data. Request minimum scopes. Handle provider errors, cancellation, replay, and callback failures safely.

### 3.3 GitHub sign-in

Implement GitHub OAuth login using a maintained library and the correct identity resolution flow.

- Retrieve the authenticated GitHub user from GitHub's API using the server-held access token.
- Retrieve verified email information through the appropriate endpoint and scopes when needed.
- Do not assume the public profile email is present or verified.
- Bind the GitHub account to the internal user only through the safe account-linking policy.
- Store provider credentials only if required; encrypt them at rest and minimize retention.
- Never expose provider access tokens to browser JavaScript or logs.

If GitHub's OAuth product does not provide the OIDC guarantees available from Google, document the exact trust and verification model rather than claiming it is OIDC.

### 3.4 Enterprise identity federation

Implement generic OpenID Connect provider support so tenants can configure enterprise identity providers such as:

- Microsoft Entra ID.
- Okta.
- Auth0.
- Keycloak.
- Other standards-compliant OIDC providers.

Requirements:

- Per-tenant identity-provider configuration.
- Discovery document validation and controlled metadata refresh.
- Issuer and audience validation.
- Authorization Code + PKCE.
- State and nonce validation.
- JWKS signature verification and key rotation.
- Strict redirect URI allowlists.
- Tenant-specific identity-provider and issuer binding.
- Configurable claims mapping with safe defaults.
- Verified email and immutable subject handling.
- Optional Just-In-Time (JIT) provisioning controlled by tenant policy.
- Optional SCIM 2.0 provisioning as a separately scoped capability if the repository can support it.
- Disable, rotate, and test provider configuration without affecting unrelated tenants.
- Never allow a user to select an arbitrary issuer URL at login and thereby create an SSRF or trust-confusion vulnerability.

For SAML 2.0, do not build an ad hoc parser. If SAML is required, use a reputable maintained implementation, strict metadata/signature validation, replay protection, and a separate documented implementation path. Do not claim SAML support unless end-to-end tests prove it.

### 3.5 Passkeys and WebAuthn

Implement passwordless passkeys using a maintained WebAuthn library where supported.

- Registration and authentication ceremonies.
- Challenge generation, single-use storage, and expiry.
- Origin and RP ID validation.
- User verification policy.
- Credential public-key and counter/backup-state handling according to library guidance.
- Multiple credentials per user.
- Credential labels, last-used time, and revocation.
- Recovery strategy that does not weaken account security.
- Cross-device and platform authenticator support.

Never store passkey private keys; they remain with the authenticator.

### 3.6 Multi-factor authentication

Implement TOTP MFA with secure enrollment and recovery:

- Enrollment QR provisioning URI.
- Confirm setup by validating a generated code before activation.
- Encrypt TOTP secrets at rest with a managed key/envelope encryption design.
- Recovery codes generated with cryptographically secure randomness, shown once, stored as hashes, single-use, and revocable.
- MFA challenge after primary authentication.
- Rate limits and replay prevention.
- MFA reset/recovery with strong identity verification and audit events.
- Require MFA for privileged roles and allow tenant policy to require it for all members.

Support WebAuthn as a phishing-resistant factor. Do not implement SMS as the default or strongest MFA. If SMS is ever added, make its limitations explicit and require anti-abuse controls.

### 3.7 API keys, SMTP credentials, and machine authentication

Preserve and harden existing API-key support.

- Generate secrets with a cryptographically secure random generator.
- Display the full secret only once.
- Store only a strong keyed hash or secure verifier; never store plaintext.
- Include key prefix/identifier for lookup without exposing the secret.
- Scope keys to tenant, owner, permissions, and optional resource restrictions.
- Support expiration, rotation, revocation, last-used tracking, and audit history.
- Use constant-time secret comparison where applicable.
- Enforce rate limits and quotas.
- Prevent API keys from being used as browser login sessions.
- Provide distinct service-account credentials and machine-to-machine OAuth client credentials only if the platform architecture needs them.
- SMTP AUTH credentials must be independently scoped, revocable, rate-limited, and never reused as dashboard passwords.
- Clearly distinguish user sessions, API keys, SMTP credentials, and service tokens.

---

## 4. Session and token security

Choose and document the correct session architecture for this application. Prefer opaque, server-managed browser sessions when practical. If JWTs are used for API or federation boundaries, keep them short-lived and validate them rigorously.

For browser sessions:

- Use `HttpOnly`, `Secure` in production, and appropriate `SameSite` cookies.
- Use host-only cookies and narrow paths; avoid broad domain cookies unless explicitly required.
- Never store long-lived access tokens, refresh tokens, or session secrets in `localStorage` or `sessionStorage`.
- Rotate session identifiers after login, privilege changes, MFA completion, and other security-sensitive transitions.
- Enforce idle and absolute expiration.
- Support server-side revocation and logout-all-devices.
- Detect and prevent session fixation.
- Implement CSRF protection for cookie-authenticated state-changing requests.
- Validate `Origin`/`Referer` as an additional defense where appropriate.
- Use strict CORS allowlists only where cross-origin access is genuinely required.
- Do not use wildcard CORS with credentials.
- Prevent open redirects; accept only allowlisted relative or registered callback destinations.
- Avoid leaking tokens in URLs, browser history, referrers, analytics, logs, and error messages.

For JWT validation:

- Pin accepted algorithms; reject `none` and algorithm confusion.
- Validate issuer, audience, expiration, not-before, issued-at where applicable, subject, and token purpose.
- Validate key IDs and support safe JWKS rotation.
- Do not accept ID tokens as API access tokens.
- Do not trust unverified claims.
- Keep signing keys outside source control and rotate them with a documented process.
- Use distinct audiences and token types for distinct services.

Implement secure logout, session listing, session revocation, and revoke-all-sessions. Explain the limits of stateless token revocation if stateless tokens are used.

---

## 5. Authorization architecture

Implement authorization centrally and enforce it server-side on every protected operation. Frontend visibility is not a security boundary.

### 5.1 Role-based access control

Create a clear role model with least privilege. At minimum evaluate:

- Platform Owner / Super Admin.
- Platform Operator / Support Admin.
- Organization Owner.
- Organization Admin.
- Security Admin.
- Developer.
- Billing Admin.
- Analyst / Read-only.
- Mailbox Admin.
- Mailbox User.
- Service Account.

Do not grant every administrator global access. Separate platform-wide privileges from tenant-scoped privileges.

### 5.2 Permission model

Use explicit permissions rather than scattered role-name checks. Define permission identifiers for resources and actions, for example:

- `tenant.read`, `tenant.update`, `tenant.delete`
- `members.read`, `members.invite`, `members.update_role`, `members.remove`
- `domains.read`, `domains.create`, `domains.verify`, `domains.delete`
- `mailboxes.read`, `mailboxes.create`, `mailboxes.update`, `mailboxes.delete`
- `messages.read`, `messages.send`, `messages.delete`
- `api_keys.read`, `api_keys.create`, `api_keys.revoke`
- `usage.read`, `billing.read`, `billing.manage`
- `security.settings.read`, `security.settings.manage`
- `audit_logs.read`
- `identity_providers.manage`
- `platform.tenants.manage`
- `platform.users.manage`

Adapt these names to existing conventions. Define a permission registry, role-permission mapping, and testable policy evaluator.

### 5.3 Resource-level authorization

Enforce ownership and tenant boundaries at the data-access/service layer:

- Every tenant-owned resource query must be constrained by the authenticated tenant context.
- Never trust a tenant ID supplied only by the client.
- Validate membership and active status before establishing tenant context.
- Prevent IDOR/BOLA attacks by checking access to the exact resource.
- Ensure background jobs and async workers carry an explicit, validated tenant/service identity.
- Use database constraints and row-level security only when compatible with the existing architecture; do not rely on frontend filtering.
- Prevent cross-tenant access through search, exports, attachments, mailbox IDs, message IDs, webhook endpoints, and audit queries.

### 5.4 Policy rules

Support explicit policies for:

- Tenant membership and invitations.
- Role assignment and privilege escalation prevention.
- MFA requirements.
- SSO enforcement.
- Session duration.
- API-key creation and scopes.
- Sensitive actions and re-authentication.
- Account suspension and tenant suspension.
- Support access and break-glass access.

Implement deny-by-default behavior. Missing policy data must not grant access.

---

## 6. Tenant onboarding, invitations, and account lifecycle

Implement secure flows for:

- Creating an organization/tenant where product policy permits.
- Inviting members by email.
- Single-use, expiring invitation tokens.
- Invitation acceptance and account creation/linking.
- Role assignment by authorized members only.
- Invitation resend, revoke, and expiration.
- Member suspension, removal, and reactivation.
- Ownership transfer with strong re-authentication and audit history.
- User deactivation and account deletion according to retention policy.
- Tenant suspension and recovery.
- Preventing the last owner from being removed without a valid ownership transfer.

Do not allow self-service registration to create privileged platform roles.

---

## 7. Frontend authentication experience

Inspect the existing Next.js App Router application and integrate authentication into its real route and component structure.

Build polished, accessible, responsive experiences for:

- Sign in.
- Create account, if enabled.
- Email verification.
- Forgot/reset password.
- Google sign-in.
- GitHub sign-in.
- Enterprise SSO discovery and redirect.
- MFA challenge and enrollment.
- Passkey enrollment and sign-in.
- Recovery codes.
- Invitation acceptance.
- Session/device management.
- Connected identity providers and account linking.
- Security settings.
- Organization member and role management.
- Unauthorized, expired-session, and provider-error states.

Frontend requirements:

1. Use server-side session validation and route protection appropriate to Next.js App Router.
2. Do not treat middleware or hidden navigation as the only authorization enforcement; backend checks remain authoritative.
3. Never expose client secrets, signing keys, provider refresh tokens, or privileged API credentials to the browser.
4. Avoid storing sensitive tokens in browser storage.
5. Use safe post-login redirect handling.
6. Handle loading, errors, expired sessions, network failures, and retry states.
7. Ensure accessible labels, keyboard navigation, focus management, clear error messages, and screen-reader status announcements.
8. Keep the design consistent with the existing SMTP product design system.
9. Do not create non-functional provider buttons or fake authentication success.
10. Do not display providers as available unless backend configuration and callback handling are operational.

If the frontend currently uses the same-origin `/api/backend/*` proxy, preserve and correctly secure that boundary unless repository inspection proves a better architecture is needed. Do not introduce unnecessary browser-to-backend cross-origin requests.

---

## 8. Backend API and data model

Design APIs consistent with existing routing, versioning, error formats, and OpenAPI documentation. Use the existing API framework and database conventions.

Implement or adapt endpoints for the following capabilities where appropriate:

- Registration and login.
- Logout and logout-all.
- Current authenticated user and active tenant context.
- Email verification and resend.
- Password reset and password change.
- OAuth/OIDC initiation and callback.
- Enterprise identity-provider configuration and discovery.
- Account linking and unlinking.
- MFA enrollment, confirmation, challenge, disable, and recovery.
- Passkey registration and authentication.
- Session listing and revocation.
- Invitation lifecycle and membership management.
- Roles, permissions, and policy management.
- API-key lifecycle and service identities.
- Audit-event retrieval with authorization.

Do not blindly use these names as exact route contracts; follow existing API conventions and avoid duplicate endpoints.

Database requirements:

- Add reversible, versioned migrations.
- Add appropriate unique constraints and indexes.
- Model users, external identities, sessions, verification tokens, password-reset tokens, MFA factors, recovery codes, passkey credentials, tenant memberships, roles, permissions, invitations, identity-provider configurations, and audit events as appropriate.
- Store token verifiers/hashes rather than raw one-time tokens.
- Encrypt secrets requiring recovery (such as TOTP/provider refresh secrets) with a managed key strategy.
- Include creation, update, expiry, revocation, and last-used timestamps where meaningful.
- Define cascade/restrict behavior intentionally.
- Avoid storing unnecessary provider profile information.

Keep OpenAPI/specification and generated clients synchronized with actual behavior.

---

## 9. Security controls and threat model

Create a threat model covering at least:

- Credential stuffing and brute force.
- Account enumeration.
- Password reset abuse.
- OAuth state/nonce/PKCE bypass.
- Account-linking takeover.
- Token replay and theft.
- Session fixation and hijacking.
- CSRF and XSS implications.
- Open redirect.
- SSRF through identity-provider metadata/discovery.
- JWT algorithm/key confusion.
- MFA bypass and recovery abuse.
- Passkey challenge replay.
- Privilege escalation.
- IDOR/BOLA and cross-tenant data exposure.
- API-key leakage and over-scoping.
- Timing side channels.
- Race conditions in token consumption and revocation.
- Insider/support access abuse.
- Audit log tampering or sensitive-data leakage.

Implement layered defenses:

- Per-IP, per-account, per-tenant, and per-endpoint rate limiting.
- Progressive throttling and abuse monitoring.
- Generic authentication failure responses.
- Secure headers and a considered Content Security Policy.
- Input validation and strict redirect validation.
- Secrets redaction in logs and traces.
- No passwords, access tokens, refresh tokens, cookies, reset links, MFA codes, or full API keys in logs.
- Safe error handling without stack traces or internal details in production.
- TLS enforcement in production and secure proxy/header trust configuration.
- Dependency and configuration security checks.
- Constant-time comparisons for secret verifiers where appropriate.
- Atomic single-use token consumption to prevent race/replay.
- Secure handling of clock skew and provider key rotation.

Do not invent cryptographic primitives. Use vetted standard libraries and document algorithms/parameters.

---

## 10. Audit logging, monitoring, and alerting

Record security-relevant events, including:

- Login success/failure.
- Logout and session revocation.
- Password change/reset.
- Email verification.
- MFA enrollment, challenge, reset, and recovery-code use.
- Passkey registration/removal/use.
- OAuth/SSO login and provider errors.
- Account linking/unlinking.
- Invitation and membership changes.
- Role/permission changes.
- API-key creation, rotation, and revocation.
- Identity-provider configuration changes.
- Tenant suspension and ownership transfer.
- Privileged support/break-glass access.

Audit records should include actor, tenant context where relevant, event type, target, timestamp, outcome, correlation/request ID, and safe contextual metadata.

Never log credentials or sensitive tokens. Protect audit records from unauthorized modification and deletion. Define retention and access controls.

Add useful metrics and alerts for unusual authentication failures, suspicious access patterns where feasible, MFA resets, privilege changes, and provider outages. Avoid collecting unnecessary personal data.

---

## 11. Configuration and secrets management

Create documented environment/configuration settings for:

- Public application origin.
- Allowed callback/redirect origins.
- Session cookie name and secure attributes.
- Session idle and absolute TTL.
- Password hashing parameters.
- Token expiry policies.
- Google OAuth client ID/secret.
- GitHub OAuth client ID/secret.
- OIDC provider configuration.
- Encryption and signing key references.
- Rate-limit settings.
- Email delivery integration for verification/reset/invitations.
- WebAuthn RP ID and allowed origins.
- MFA policy defaults.

Requirements:

- Provide safe local development configuration without committing real secrets.
- Validate required production configuration at startup.
- Fail closed when a security-critical configuration is missing or invalid.
- Keep secrets in environment/secret manager, never source control.
- Provide rotation procedures and key-version support.
- Separate development, staging, and production credentials.
- Never print secret values during startup or diagnostics.

---

## 12. Email delivery integration

Authentication emails must use the platform's real email delivery abstraction, not a fake success response.

Implement secure, clear templates for:

- Email verification.
- Password reset.
- Invitation.
- Security notification.
- New login/device notification where policy supports it.

Requirements:

- High-entropy single-use links.
- Short, documented expiration windows.
- No sensitive token logging or analytics capture.
- Safe link construction from configured canonical origin, not an untrusted Host header.
- Rate limits and resend cooldowns.
- Clear handling when delivery is delayed or fails.
- Avoid disclosing account existence.

If outbound delivery is not operational in the current repository, implement the integration boundary and report the exact dependency/limitation. Do not claim emails are delivered when they are only queued or simulated.

---

## 13. Testing requirements

Add meaningful automated tests at unit, integration, API, and frontend levels using the repository's existing test stack.

### Authentication tests

- Correct and incorrect password.
- Password hash verification and upgrade path.
- Account enumeration resistance.
- Verification/reset token expiry, single-use, and replay.
- OAuth state, nonce, PKCE, issuer, audience, signature, and redirect validation.
- Provider failure and key rotation.
- Safe account linking and duplicate identity handling.
- MFA enrollment/challenge/recovery and brute-force limits.
- WebAuthn challenge/origin/RP ID validation.
- Session rotation, expiry, revocation, logout-all, and CSRF protection.
- Cookie attributes in production configuration.

### Authorization tests

- Every role/permission mapping.
- Deny-by-default behavior.
- Tenant isolation across all tenant-owned endpoints.
- Cross-tenant IDOR/BOLA attempts.
- Role escalation attempts.
- Suspended user and suspended tenant behavior.
- API-key scope and revocation.
- Service identity restrictions.
- Background job tenant context.
- Last-owner protection.

### Security and reliability tests

- Concurrent use of one-time tokens.
- Rate limiting.
- Malformed and oversized input.
- Provider timeout/unavailability.
- Database transaction rollback.
- Secret redaction.
- Migration up/down or repository-standard migration validation.
- Regression tests for existing API-key and service flows.

### Frontend tests

- Authenticated and unauthenticated route behavior.
- Provider error states.
- MFA and recovery flows.
- Expired session behavior.
- Role-based UI visibility as usability only, with backend authorization tested separately.
- Keyboard and accessible form behavior.

Use deterministic test providers or local test fixtures. Never use real production OAuth credentials in tests. Do not weaken production validation to make tests pass.

---

## 14. Documentation and operational readiness

Update:

- Architecture and identity-flow diagrams.
- Authentication and authorization API documentation.
- OpenAPI specification.
- Environment variable reference.
- Local setup guide.
- Google OAuth setup guide.
- GitHub OAuth setup guide.
- Enterprise OIDC setup guide.
- WebAuthn/passkey origin and RP ID guide.
- MFA and recovery guide.
- Session and cookie security guide.
- RBAC/permission matrix.
- Tenant isolation model.
- Threat model and security decisions.
- Key rotation and incident response runbooks.
- Production deployment checklist.
- Known limitations and provider support matrix.

Include exact callback URLs derived from the actual configured routes. Never invent callback paths without implementing them.

---

## 15. Implementation sequence

Implement in dependency order. Adapt numbering if repository realities require it, but preserve the dependency sequence.

### Stage A — Discovery and security design
- Repository audit.
- Existing identity and authorization inventory.
- Threat model.
- Identity architecture and data-flow design.
- Security decision records.

### Stage B — Core identity foundation
- User and external identity models.
- Password hashing and first-party identity service.
- Token generation/verification primitives using vetted libraries.
- Database migrations and constraints.
- Authentication middleware/context.
- Generic error and audit foundations.

### Stage C — Sessions and account lifecycle
- Secure session persistence and cookies.
- Login/logout/revocation.
- Registration policy.
- Email verification.
- Password reset/change.
- Rate limiting and CSRF protections.

### Stage D — OAuth and OIDC federation
- Google login.
- GitHub login.
- Generic OIDC provider abstraction.
- Enterprise provider configuration.
- Safe identity linking.
- JIT provisioning policy.

### Stage E — MFA and passwordless
- TOTP.
- Recovery codes.
- WebAuthn/passkeys.
- Privileged-user MFA enforcement.
- Recovery and reset controls.

### Stage F — Authorization and tenancy
- Permission registry.
- Roles and role assignments.
- Policy evaluator.
- Resource-level authorization.
- Tenant isolation enforcement.
- Member invitation and lifecycle.
- Platform-vs-tenant admin separation.

### Stage G — Machine identities
- API-key hardening.
- Service accounts.
- SMTP credential boundaries.
- Scopes, expiry, rotation, revocation, quotas.

### Stage H — Frontend integration
- Auth routes and flows.
- Session-aware application shell.
- SSO and provider UX.
- MFA/passkey UX.
- Session and security settings.
- Member/role management.

### Stage I — Observability and hardening
- Audit events.
- Metrics and alerts.
- Security headers and production config.
- Abuse controls.
- Secret rotation and runbooks.

### Stage J — Verification and release readiness
- Full automated test suite.
- Migration verification.
- Lint, formatting, type checking, build.
- API contract validation.
- Security regression review.
- Documentation and final status report.

Do not implement all providers at once if the foundational security model is not ready. Complete and test each stage before building on it.

---

## 16. Engineering rules

- No mock authentication in production code.
- No fake OAuth callback success.
- No hardcoded credentials, secrets, users, or bypass tokens.
- No default admin password.
- No insecure development bypass in production builds.
- No trusting email alone for account linking.
- No authorization enforced only in React components.
- No tenant ID trust from request body/query without membership validation.
- No wildcard credentialed CORS.
- No `no-cors` workaround.
- No long-lived browser tokens in local storage.
- No plaintext secrets or recoverable password storage.
- No custom cryptographic algorithms.
- No silent catch blocks or swallowed auth errors.
- No broad refactor unrelated to authentication unless required and justified.
- No breaking existing API-key behavior without a documented migration path.
- No claims of provider support until end-to-end tests pass.
- No claims of production readiness while required email, secret management, TLS, or provider configuration is missing.

Keep code modular, typed, testable, observable, and consistent with repository conventions. Avoid unnecessary abstractions, but isolate provider-specific logic behind stable interfaces.

---

## 17. Required validation

Run the repository's actual validation commands. At minimum, inspect and run all applicable commands for:

- Formatting.
- Linting.
- Static analysis/type checking.
- Unit tests.
- Integration/API tests.
- Frontend tests.
- Database migration validation.
- Production builds.
- Security/dependency scans if configured.

For the current workspace, verify actual package scripts before choosing commands. If frontend scripts exist, run relevant dashboard checks such as:

- `pnpm lint:dashboard`
- `pnpm typecheck:dashboard`
- `pnpm build:dashboard`

Run the backend's actual formatter, linter, test, and build commands discovered from its manifests and CI configuration.

Do not report a check as passed unless it was actually run and passed. If a check cannot run, state why and what remains.

---

## 18. Definition of done

The implementation is complete only when:

- Users can authenticate through every provider explicitly marked supported.
- OAuth/OIDC flows validate all required protocol protections.
- Account linking cannot be used to take over another account.
- Sessions are secure, revocable, and correctly expire.
- Password reset and verification are single-use and abuse-resistant.
- MFA and passkey flows work end to end where marked supported.
- Authorization is enforced by backend services for every protected operation.
- Tenant isolation is tested against cross-tenant access attempts.
- Roles and permissions are documented and least-privilege by default.
- API keys, SMTP credentials, and service identities are distinct and properly scoped.
- Frontend flows are accessible, responsive, and connected to real backend behavior.
- Security events are auditable without leaking secrets.
- Configuration and operational runbooks are complete.
- Tests, lint, type checks, migrations, and production builds pass, or outstanding blockers are explicitly documented.
- No fake functionality, insecure bypasses, or unsupported provider claims remain.

---

## 19. Final report required

At completion, provide a concise but specific report containing:

1. Architecture implemented.
2. Authentication methods fully supported, partially supported, and not supported.
3. Authorization roles, permissions, and tenant enforcement implemented.
4. Database migrations and API routes added/changed.
5. Frontend routes and components added/changed.
6. Security controls implemented.
7. Tests and exact validation commands with real results.
8. Required production environment variables and provider setup.
9. Remaining blockers, risks, and follow-up work.
10. Any existing behavior that could not safely be preserved.

**Start by inspecting the repository. Then implement the complete authentication and authorization foundation in dependency order. Do not stop at a proposal or a login page.**
