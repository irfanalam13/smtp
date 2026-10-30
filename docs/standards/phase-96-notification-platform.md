# Phase 96 — Notification Platform

Status: **No centralized preference, delivery, or notification tracking service implemented**

## Repository baseline

The platform has SMTP message delivery foundations, API events/outbox
primitives, and a proposed webhook system. It has no centralized
email/SMS/push/web notification service, user preference model, notification
template catalog, or delivery tracking across channels.

## Recommended notification model

Separate product notifications (security, billing, invitations) from customer
mail delivery and outbound marketing. Use typed events, tenant/user
preferences, consent and legal basis, channel eligibility, deduplication,
priority, expiry, quiet hours, locale, and idempotent delivery records.
Security and account-recovery notifications need protected minimum delivery
behavior and cannot be silently disabled where policy or law requires them.

Use maintained channel providers behind adapters. Keep credentials in a
secret manager and apply provider rate limits, retry/backoff, bounce/complaint
suppression, and opt-out semantics. Push and web notifications require
explicit device/origin registration and revocation. Slack/Teams/webhook
notifications reuse the integration authorization model. Do not treat
transactional SMTP or customer webhooks as generic notification transport.

## Remaining work and why

Define notification classes, consent, preferences, fallback/escalation,
channels/providers, rate limits, retention, templates, localization,
suppression, status model, and delivery SLOs. Implement an asynchronous
idempotent dispatcher with channel-specific adapters, preference APIs, audit,
and provider failure testing.

This remains open because the repository lacks notification product semantics
and provider credentials/contracts. Channels differ in consent, delivery
receipts, security, and retry behavior; a single generic `send` path could
violate opt-out rules or misreport delivery.

## Acceptance criteria

- Notification intent, attempt, and delivery state are distinct and
  idempotent.
- Consent/preferences are checked at send time, with documented mandatory
  security exceptions.
- Provider outages, retries, rate limits, bounce/complaint, unsubscribe, and
  device revocation are tested.
- Secrets and notification payloads are minimized and redacted in logs.
- Notifications never block SMTP acceptance/delivery or expose another
  tenant's identity/data.

## Research references

- [Phase 81 Webhook System](phase-81-webhook-system.md)
- [Phase 83 Email Templates](phase-83-email-templates.md)
- [Phase 85 Public Integrations](phase-85-public-integrations.md)
