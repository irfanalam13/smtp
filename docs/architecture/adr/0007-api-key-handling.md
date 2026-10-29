# ADR 0007: API-key handling

## Status

Accepted for Milestone 4.

## Decision

API keys are generated with the operating system CSPRNG. The bearer secret is
returned only by the create response, while the store retains a non-secret
prefix and SHA-256 verifier for the high-entropy token. Listings expose only
safe metadata. Keys support scopes, expiry, and revocation. The API never puts
secrets in URLs, logs, or persistent browser state.

The current API-key store is an in-memory local-development adapter. The
versioned PostgreSQL schema is ready for the production repository
implementation; a production deployment must not silently use the in-memory
adapter.
