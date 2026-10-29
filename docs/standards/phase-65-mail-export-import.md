# Phase 65 — Mail Export and Import

Status: **Canonical raw messages are stored; interoperable import/export tools are not implemented**

## Research and contract

EML is a single RFC 5322 message. MBOX is a family of mailbox container
conventions and requires correct `From ` separator escaping; it is not one
uniform format. PST is a Microsoft-specific binary format and should be
handled through a separately selected, licensed/maintained conversion tool,
not an ad-hoc parser in the mail store.

## Implementation and remaining

Messages retain canonical raw bytes in PostgreSQL. There are no export/import
endpoints, streaming archive jobs, MBOX variants, PST conversion, manifest,
dry-run validation, or migration checkpoint/retry tools.

## Acceptance criteria

- Import validates size, MIME, message count, duplicate policy, and target
  mailbox ownership before committing each bounded batch.
- Export is streamed, auditable, authorization-checked, and preserves raw
  bytes.
- MBOX format variant and PST conversion tool are explicit product choices.

## Sources

- [RFC 5322 — Internet Message Format](https://www.rfc-editor.org/rfc/rfc5322.html)
- [RFC 4155 — The application/mbox Media Type](https://www.rfc-editor.org/rfc/rfc4155.html)
