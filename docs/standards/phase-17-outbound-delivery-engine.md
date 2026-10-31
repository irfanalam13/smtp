# Phase 17 — Outbound Delivery Engine

Status: **Recipient-aware delivery implementation started; production profile
and acceptance testing remain incomplete**

The delivery component already performs bounded MX resolution, destination
safety checks, SMTP greeting/EHLO, STARTTLS policy, envelope delivery, DATA
dot-stuffing, reply parsing, and candidate failover.

The queue now persists per-recipient delivery/attempt rows, partitions claims
by destination domain, applies database-backed per-domain leases, retries
temporary outcomes independently, and marks expired in-flight sends uncertain
instead of risking duplicate mail. A real `mail-delivery` worker connects
the queue to the SMTP client, revalidates schema-versioned MIME data, records
individual RCPT/DATA outcomes, and drains in-flight work on shutdown. The SMTP
client routes mixed-domain inputs independently, uses an exact-domain
allowlist, validates/pins resolved addresses, handles explicit null MX and
implicit MX fallback, and preserves recipient-level SMTP status/diagnostics.
Remote `2xx` acceptance is clearly distinguished from inbox placement.

**Not complete:** PostgreSQL migration and integration tests must be executed
against an isolated database before the durable path is accepted. A complete
production profile still needs lease renewal and lease-duration coupling,
tenant/domain TLS policy, IP-level throttling, destination circuit breakers,
connection reuse policy, delivery telemetry, broader destination-policy
review, and external SMTP-client interoperability evidence. MTA-STS, TLS-RPT,
DANE and authenticated smarthost support are not implemented. Do not claim
M05 production readiness until these gaps are implemented or explicitly
accepted/deferred with operational controls.
