# Transactional email platform product scope

**Research snapshot:** 2026-10-30

## Product direction

Build a developer-first transactional email service around a reliable SMTP/API
delivery core. The first product profile is not a full marketing CRM. A
capability belongs in the customer dashboard only when its API, persistence,
authorization, and operational behavior exist and can be verified end to end.

The interface should make the message lifecycle understandable:

1. Configure and verify a sender domain.
2. Authenticate with a scoped API credential or SMTP credential.
3. Submit an email over an API or authenticated SMTP.
4. Inspect queue, attempt, and eventual remote-delivery outcomes.
5. Receive trustworthy event notifications and act on bounces or complaints.

Queue acceptance, remote SMTP acceptance, mailbox placement, and engagement
are distinct states. The product and its analytics must not conflate them.

## Market reference points

This is a capability comparison, not a feature-copying specification. Vendor
feature claims below refer to their public product or developer documentation
as reviewed on the research date.

| Platform                                                                     | Useful product pattern                                                                                                                                                                                                                                        |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Brevo](https://www.brevo.com/products/marketing-platform/)                  | A broader customer-engagement suite: marketing campaigns and automation alongside transactional communication. This is a useful future expansion reference, not the initial product scope.                                                                    |
| [Twilio SendGrid Email API](https://www.twilio.com/en-us/products/email-api) | Developer-oriented sending APIs and SMTP relay, with delivery and engagement tracking. Its [Event Webhook reference](https://www.twilio.com/docs/sendgrid/for-developers/tracking-events/event) illustrates the importance of explicit, typed message events. |
| [Postmark](https://postmarkapp.com/)                                         | A developer-focused delivery experience that distinguishes transactional and broadcast message streams. Its [webhook documentation](https://postmarkapp.com/developer/webhooks/webhooks-overview) covers delivery, bounce, and other event handling.          |
| [Mailgun](https://www.mailgun.com/email-api/)                                | Email APIs with message activity, delivery diagnostics, analytics, and deliverability-oriented tooling.                                                                                                                                                       |
| [Amazon SES](https://docs.aws.amazon.com/ses/latest/dg/Welcome.html)         | A mail transport foundation exposed through both SMTP and APIs, with event integration through services such as SNS and CloudWatch.                                                                                                                           |

The consistent baseline is not a large navigation menu. It is an accurate
message lifecycle, reliable sending, clear domain authentication, actionable
failure information, secure event delivery, and honest operational status.

## Customer-facing page map

“Available now” means backed by the current application, not production
complete. “Next” is the implementation order; later items must pass the
dependencies and release gates below before being advertised.

| Page / capability                      | Available now                                                                                                 | Product gap and intended behavior                                                                                                                                                                         |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Overview                               | Recent submissions, domain/key/mailbox counts, and a usage view.                                              | Usage is process-local; add durable tenant reporting, time ranges, delivery outcomes, and clear empty/error states.                                                                                       |
| Send email                             | Plain-text API submission into the PostgreSQL queue.                                                          | Acceptance is not delivery. Add authenticated SMTP submission, production worker integration, MIME/HTML/templates, recipient-level outcomes, and safe retry visibility.                                   |
| Activity                               | Recent queue state is shown in existing views; the first activity slice adds persisted queue-attempt history. | Real outbound SMTP results are now persisted per recipient, but the activity API/dashboard still needs a tenant-scoped delivery read model and durable event timeline before exposing delivery activity.     |
| Domains                                | Register a domain, issue/check an ownership challenge, and run bounded DNS diagnostics.                       | Add operational DKIM key creation/rotation, SPF/DMARC guidance and enforcement evidence, domain lifecycle controls, and monitored readiness.                                                              |
| API keys                               | Create and revoke scoped API keys; a new secret is shown once.                                                | Add last-used metadata and API-key expiry controls. SMTP credentials have a separate tenant-scoped lifecycle API with one-time secrets and audit events. SMTP AUTH is implemented in the ingress service. |
| Mailboxes                              | Browse metadata for provisioned inbound mailboxes.                                                            | This is not a hosted inbox product yet. Complete authorization, storage lifecycle, search, retention, and supported mailbox protocols before expanding the UI.                                            |
| Webhooks / events                      | Not implemented.                                                                                              | Provide event subscriptions, signing secrets, delivery attempts, replay, deduplication, and endpoint health. Publish only events backed by durable state.                                                 |
| Templates                              | Not implemented.                                                                                              | Add versioned templates, rendering/preview, validation, and API integration after message content and MIME contracts are settled.                                                                         |
| Suppression / bounces                  | Not implemented.                                                                                              | Persist recipient suppressions and bounce/complaint events; enforce suppression at every send path and expose scoped remediation.                                                                         |
| Campaigns                              | Not implemented and outside the initial transactional profile.                                                | A future campaign product requires consent and unsubscribe records, contact management, scheduling, suppression enforcement, abuse controls, and auditable campaign state.                                |
| Team, roles, usage, billing, and audit | Partial identity, tenant membership, API-key scope, and process-local usage foundations.                      | Complete browser identity/session flows, durable RBAC, metering, billing, and searchable tenant audit events before presenting a full account administration surface.                                     |

Do not add links for templates, campaigns, event management, analytics, or
settings until each link reaches a functioning capability. An unavailable
feature may be described in product documentation but must not be presented as
an operational dashboard page.

## Implementation sequence

### 1. Trusted message submission and delivery

- Complete SMTP submission authentication and tenant/domain authorization.
- Connect the durable queue worker to the bounded outbound SMTP transport.
- Preserve per-recipient attempts, remote SMTP response class, retry schedule,
  and terminal failure state without logging message bodies or credentials.
- Keep external sending disabled by default outside an explicitly configured
  environment; prove safe behavior with controlled SMTP integration fixtures.

### 2. Delivery operations and event data

- Persist immutable, tenant-scoped lifecycle events for accepted, attempted,
  deferred, delivered-to-next-hop, bounced, and complained outcomes.
- Distinguish remote-server acceptance from inbox placement and engagement.
- Add paginated activity search, message detail, recipient outcomes, and
  retention limits; expose only sanitized diagnostics to authorized users.
- Add signed, retryable, idempotent webhooks with endpoint health and replay.

### 3. Sender setup and developer experience

- Finish authenticated browser access, tenant switching, and account lifecycle.
- Add SMTP credentials, API examples/SDKs, domain authentication setup, and
  status/readiness explanations based on actual backend evidence.
- Add templates after the API and SMTP message/MIME contracts support them.

### 4. Account operations and expansion

- Make usage metering durable and define rate, quota, and billing behavior.
- Add organization roles, audit history, service health, support workflow, and
  abuse controls.
- Consider campaigns only as a separately designed product area with consent,
  unsubscribe, contact, scheduling, and suppression foundations. SMS, CRM, and
  marketing automation are separate scope decisions, not assumed SMTP features.

## Definition of done for a dashboard capability

Before calling a page functional, verify all of the following:

- Every displayed field comes from an implemented API contract and durable
  source where persistence is required.
- Tenant and permission checks are enforced by the API, not merely by UI state.
- Loading, empty, invalid-input, permission-denied, and service-error states
  are explicit and useful.
- Mutations have validation, idempotency where appropriate, and observable
  results; secrets are shown once and never placed in URLs or persistent
  browser storage.
- API contract, implementation, focused tests, operations guidance, and
  dashboard behavior agree.
- The UI labels simulated, queued, attempted, and remotely accepted states
  accurately; no state claims inbox placement.

## Current release boundary

The repository is a development foundation, not a hosted production ESP. The
Go API accepts API submissions into PostgreSQL and can list queue state. The
current queue processor is a deterministic test processor; its
`simulated_succeeded` outcome is not external SMTP delivery. The current
dashboard supports overview, sending, domains, mailboxes, and API keys. The
activity page introduced with this scope exposes persisted queue attempts and
must preserve that simulation label until real transport outcomes are wired.

For code-level readiness and deferred prerequisites, see the
[project status](../project-status.md), [phase register](../phase-status.md),
[API contract](../api/README.md), and
[outbound delivery architecture](../architecture/outbound-delivery.md).
