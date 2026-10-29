# Phase 39 — Bounce and Delivery Status

## Standards baseline

SMTP reply classification follows RFC 5321 enhanced by RFC 3463 enhanced status
codes. DSNs use RFC 3464, SMTP DSN parameters are in RFC 3461, and ARF feedback
reports use RFC 5965. DSNs must not be sent in response to a message with a
null reverse-path, to avoid bounce loops. Recipient outcomes must remain
separate for multi-recipient deliveries.

## Implemented foundation

The core classifies enhanced status codes into temporary, permanent, policy,
and invalid-recipient categories; creates recipient-scoped DSN records; tracks
reports by original message ID; and parses a bounded set of ARF fields.

## Production requirements

- Persist delivery attempts and per-recipient terminal states.
- Generate standards-conformant `multipart/report; report-type=delivery-status`
  messages, including safe original-message handling.
- Enforce null-sender suppression, privacy rules, DSN NOTIFY/RET/ORCPT, and
  loop prevention.
- Authenticate and deduplicate feedback reports before updating suppression.
- Add event delivery, retention, audit, and recipient-level customer APIs.

