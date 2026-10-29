# Core infrastructure

Phase 04 extends the existing `shared-types` crate at
[`smtp-core/`](../../smtp-core/) rather than creating empty `core-*` crates.
The crate is published internally as `shared-types` and contains focused
modules:

| Module | Responsibility |
|---|---|
| `config` | Environment enum, typed baseline configuration, defaults, and validation |
| `secrets` | Validated secret keys, redacted values, and environment-backed local provider |
| `context` | Distinct bounded correlation, request, and trace identifiers |
| `errors` | Shared error class and safe infrastructure error wrapper |
| `events` | Versioned serializable envelope and publisher contract; no broker |
| `logging` | Application-owned `tracing` subscriber initialization |
| `runtime` | Explicit shutdown signal and bounded wait |

## Dependency direction

Application crates may depend on `shared-types`. The shared crate does not
depend on application services, database schemas, SMTP protocol code, or a
production broker. External libraries are declared once in the workspace
manifest. Domain-specific configuration and errors remain in their owning
service until a real cross-service contract exists.

## Startup and shutdown

Services should load and validate configuration first, initialize logging
second, initialize required dependencies third, then start workload tasks.
Required initialization failures must terminate startup. A service owns its
`Shutdown` sender and passes a `ShutdownReceiver` to background work. It must
stop accepting new work, stop consumers, flush/close resources where supported,
and respect a bounded deadline before exiting.

## Configuration contract

`PlatformConfig::from_env()` reads optional values using this precedence:

1. typed built-in development-safe defaults;
2. environment variables for `SERVICE_NAME`, `APP_ENVIRONMENT`,
   `BIND_ADDRESS`, `LOG_FILTER`, and `SHUTDOWN_TIMEOUT_MS`.

The current shared baseline does not invent file or command-line sources.
Service-specific configuration can layer on top. Production rejects loopback
binding and shutdown timeouts that are zero or greater than five minutes.
Diagnostics identify fields and constraints but never print secret values.

## Secret contract

`SecretKey` validates bounded environment names. `EnvSecretProvider` is for
local development and tests; it does not claim memory erasure or production
secret-manager integration. `SecretValue` redacts its `Debug` output and is
only exposed through an explicit `expose()` call. Missing secrets and provider
failures are distinct error cases.

## Logging and sensitive data

Executables call `shared_types::logging::init` once. Development/test use
compact output; staging/production use JSON. Libraries must use `tracing`
events without installing a global subscriber. Never log passwords, tokens,
keys, raw secret responses, message bodies, authentication headers, or
unnecessary recipient/personal data.

## Events

`EventEnvelope<T>` contains a UUID event ID, bounded event type, schema version,
UTC occurrence time, producer, optional correlation ID, and typed payload.
`EventPublisher` is a contract only. Phase 01 selected PostgreSQL
queue/outbox initially and did not select a broker, so Phase 04 does not add a
fake or non-durable adapter. A future adapter must distinguish accepted,
persisted, and replicated publication and must document retries, ordering,
acknowledgement, duplicates, poison events, and cancellation.

Events must not contain credentials, secrets, full message bodies, or
attachments. Consumers must validate type/version and implement application
idempotency using event identity.

