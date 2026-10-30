# Phase 107 — Distributed SMTP, IMAP, POP3, and API Services

Status: **Limited SMTP/API prototypes and mailbox read paths; clustered protocol services are not implemented**

## Repository baseline

The API has limited durable mailbox reads, SMTP ingress/outbound foundations,
and a development queue worker. The repository does not show production
IMAP/POP3 services or clustered protocol deployments. SMTP ingress defaults
to loopback and does not provide production TLS/AUTH. Protocol state,
mailbox writes, domain routing, and delivery are not validated across nodes.

## Recommended design

Treat protocol listeners as independently scalable stateless frontends only
when their state and authorization are externalized. Persist accepted
messages durably before returning success; use stable message IDs and
idempotency so a retry after ambiguous failure cannot silently duplicate
delivery. Drain SMTP sessions safely, bound concurrent connections, enforce
per-tenant/per-destination quotas, and isolate slow or abusive peers. Mailbox
protocol nodes must authorize every mailbox operation and share durable
metadata/object storage. Do not claim IMAP/POP3 support until protocol
conformance and interoperability tests exist.

## Remaining work and why

Complete mailbox storage/authentication/protocol implementation first; define
per-protocol load balancing, session affinity (if any), TLS, AUTH, quotas,
timeouts, backpressure, and acceptance durability. Then run interoperability,
load, and node-failure tests against persistent state. This remains open
because mailbox protocols and production identity/storage are incomplete.

## Acceptance criteria

- Accepted mail survives process/node restart and is not acknowledged before
  the configured durability point.
- Retry/timeout ambiguity is covered by idempotency and duplicate tests.
- SMTP/IMAP/POP3 conformance suites exercise concurrent sessions and
  mid-session failures.
- Tenant limits, abuse controls, TLS/auth policy, and draining work across
  every replica.
- A node failure cannot corrupt or expose another tenant's mailbox state.

## Research references

- [Phase 10 Mailbox Integration](phase-70-mailbox-platform-integration.md)
- [SMTP inbound architecture](../architecture/inbound-mail.md)
- [Data and deployment architecture](../architecture/phase-01/data-and-deployment.md)
