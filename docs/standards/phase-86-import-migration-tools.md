# Phase 86 — Import and Migration Tools

Status: **Raw mailbox data is retained; customer migration connectors and jobs pending**

## Repository baseline

Phase 65 covers EML/MBOX mailbox import/export and remains unimplemented.
The repository has inbound message persistence and mailbox metadata primitives,
but no Gmail, Outlook/Exchange, Microsoft 365, Google Workspace, cPanel, or
third-party IMAP migration job. Contact/calendar models and migration are not
implemented in the supplied phase scope.

## Recommended migration contract

Build a resumable, user-authorized migration pipeline with a source connector,
inventory/validation stage, bounded import workers, idempotent checkpoints,
and reconciliation report. Prefer provider-authorized APIs with minimum
read-only scopes; support standard IMAP only with TLS and explicit credentials
handled through a secret-provider boundary. Never ask for a provider account
password when OAuth is available.

Preserve raw message bytes and relevant source metadata. Map folders, flags,
labels, timestamps, attachments, and duplicates through an explicit
source-to-target mapping; report lossy conversions. Apply message-size,
concurrency, rate, quota, and malware scanning limits. Treat contacts,
calendars, archives, and delegated mailboxes as separate data products with
their own models and consent.

## Remaining work and why

Specify supported source providers and versions, permissions/consent, source
inventory, mapping and duplicate semantics, checkpoints, cancellation,
backfill/rate-limits, audit, privacy, error reports, and customer rollback.
Implement one connector and a generic migration job contract, then validate
with synthetic and customer-authorized test mailboxes.

This remains open because no source provider, migration scale, or
contact/calendar product has been selected. Migration can create duplicates,
lose metadata, exceed quotas, or expose highly sensitive historical data;
provider-specific behavior and legal consent must be designed before jobs
are exposed.

## Acceptance criteria

- Jobs are resumable and idempotent across process/database failure.
- Source credentials are least-privilege, encrypted, never logged, and
  revocable; job cancellation terminates access.
- Mapping, skipped/failed items, duplicate handling, and partial completion
  are reported and auditable.
- Tenant quota, scanning, retention, and data residency apply during import.
- Representative migration fixtures prove raw-message integrity and expected
  folder/metadata mapping.

## Research references

- [Phase 65 Mail Export and Import](phase-65-mail-export-import.md)
- [Gmail API documentation](https://developers.google.com/workspace/gmail/api)
- [Microsoft Graph documentation](https://learn.microsoft.com/graph/)
- [RFC 9051 — IMAP4rev2](https://www.rfc-editor.org/rfc/rfc9051.html)
