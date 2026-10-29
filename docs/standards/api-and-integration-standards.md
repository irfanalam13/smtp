# API and integration standards

## Public management API

Phase 01 selects versioned REST/JSON with OpenAPI. HTTP semantics are governed by the selected framework and current HTTP specifications; JSON uses RFC 8259. The API contract must define:

- tenant-scoped authentication and authorization;
- resource versioning and deprecation;
- idempotency keys for message submission and mutating retries;
- bounded pagination and filtering;
- stable error categories with correlation IDs;
- rate limits and `Retry-After` behavior;
- audit events for credential, domain, policy, and administrative changes.

OpenAPI is a contract format, not a guarantee of HTTP correctness. Phase 08 owns the concrete OpenAPI document and compatibility rules.

## OAuth and OIDC

The architecture uses OIDC for operator/API user identity and scoped API keys for programmatic service access. OAuth 2.0/OIDC deployment must select an issuer, audience, authorization-code/client-credentials use cases, token lifetimes, key discovery, rotation, and revocation behavior. RFC 6749, RFC 6750, RFC 8414, and the OpenID Connect Core specification are references for the later identity design; no custom token issuer is introduced.

SMTP OAuth SASL (RFC 7628) is separate from API OIDC. It is optional until the SMTP AUTH mechanism and provider/client compatibility requirements are selected.

## Webhooks and events

There is no single universal webhook RFC that replaces platform design. Customer webhooks use HTTPS, signed requests with rotating tenant secrets, timestamp/replay windows, bounded payloads, explicit event IDs, idempotent consumer guidance, retry/backoff, and a dead-letter state. Event envelopes follow the Phase 01 fields and never carry raw message bodies or credentials by default.

## External integrations

DNS providers, billing, malware scanning, reputation feeds, monitoring, and identity providers are adapter boundaries. Each adapter has timeouts, circuit breakers, least privilege, request signing or TLS validation, data minimization, auditability, and a failure policy. An external API being available does not make it an Internet standards dependency.
