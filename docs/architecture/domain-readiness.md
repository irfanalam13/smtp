# Domain readiness and sending controls

The control plane exposes bounded, authenticated DNS diagnostics for registered
domains. Checks use the system resolver with a three-second timeout and are
injected in tests. The current implementation reports ownership state, SPF TXT
presence/multiplicity, DMARC TXT presence, and MX presence. SPF and DMARC
results are advisory observations, not complete policy evaluation or inbox
placement guarantees. MX is advisory because inbound receiving is optional.

Results are cached in the current control-plane store for 15 minutes and
requests are limited to six checks per tenant per minute. A timeout is
`pending`, never `pass`; stale results are surfaced as stale/checking. The
dashboard never edits DNS.

Tenant usage currently uses a conservative one-hour, single-process window of
1,000 message submissions and a 10 MiB maximum message size. This is not a
distributed quota implementation because the existing Go control plane is an
in-memory development adapter. Production must move accounting into the
PostgreSQL repository before multi-instance operation.

Inbound authentication and outbound signing are separate from these advisory
diagnostics. The M07 SMTP ingress evaluates SPF/DKIM/DMARC, and outbound
delivery can sign with Ed25519 keys mounted through `SMTP_DKIM_KEY_DIR`. The
control plane still does not provide tenant DKIM key creation, rotation, or
selector lifecycle endpoints.
