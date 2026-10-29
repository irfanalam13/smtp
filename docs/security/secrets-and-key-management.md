# Secrets and key management

Secrets must be supplied through the existing Phase 04 secret abstraction or
an approved production secret manager. Secret values must not appear in logs,
debug output, URLs, audit metadata, or API-key listings.

The current control-plane API-key adapter stores only a verifier and prefix in
memory. It does not expose the original secret after creation. Production
storage must use a database transaction, access-controlled columns, rotation
and revocation operations, and a migration plan.

Signing keys, TLS private keys, encryption keys, database credentials, and API
keys are separate classes of secret and must not share material. Production
key ownership, rotation schedules, emergency revocation, backups, and access
reviews remain deployment decisions.
