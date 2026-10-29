# Security audit logging

Security-sensitive control-plane operations emit structured audit events with
tenant, actor, action, target type, target identifier, metadata, and UTC time.
Audit metadata must never contain passwords, bearer tokens, private keys, full
API-key secrets, or raw message bodies.

The current development control-plane adapter retains events in process memory.
This is useful for local behavior and tests but is not a durable production
audit guarantee. A production adapter must write to the authoritative
PostgreSQL audit table and forward to an access-controlled, append-oriented
sink without making sensitive data available to ordinary tenants.

Authentication failures should remain generic to clients while retaining only
safe operational context for security monitoring.
