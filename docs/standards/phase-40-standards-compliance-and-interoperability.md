# Phase 40 — Standards Compliance and Interoperability

## Scope and evidence

This phase is a continuing release gate, not a one-time checklist. Conformance
requires positive and negative protocol fixtures, parser fuzzing, resource and
security limits, database failure/recovery tests, and interoperability evidence
for the exact deployed profile. A passing unit suite is not provider
certification.

## Implemented foundation

The core includes a small deterministic fixture runner for expected-versus-
actual behavior. Existing SMTP tests exercise selected protocol transitions,
delivery framing, TLS behavior, and destination safety. The repository has
standards traceability and conformance planning documents.

## Release gate still required

- Add RFC-indexed fixtures for every advertised SMTP/IMAP/POP3 feature.
- Add malformed-input, fuzz, load, concurrency, failover, and recovery suites.
- Run controlled interoperability against approved test accounts and systems.
- Capture provider-specific requirements separately from RFC requirements.
- Validate deployment configuration, key rotation, backups, observability,
  abuse controls, data retention, and incident response.
- Record results by commit, profile, test environment, and date before making
  compliance or production-readiness claims.

No major-provider interoperability run or production certification is claimed
by the current implementation.

