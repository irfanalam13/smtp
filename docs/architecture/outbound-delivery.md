# Outbound SMTP delivery

`smtp-delivery` is the controlled outbound transport for queued
messages. The `mail-delivery` executable claims per-domain recipient batches
from `mail-queue`, revalidates the stored schema-versioned canonical message,
and submits the exact stored raw bytes. Queue persistence owns leases, retries,
recipient outcomes, and parent message states.

Delivery is disabled unless `SMTP_DELIVERY_ENABLED=true` and a recipient
domain allowlist are explicitly configured. The allowlist uses exact normalized
domain matching. Production delivery resolves
MX records, sorts candidates by preference, resolves each MX host to address
records, filters unsafe addresses, and connects to the validated address
directly. The client does not perform a second hostname-based connection after
the safety check.

The SMTP dialogue is bounded at every stage: connection, commands, replies,
message bytes, MX candidates, and fallback attempts. Replies are parsed as
bounded single- or multi-line SMTP responses. Message bytes are normalized to
CRLF, dot-stuffed, and terminated with the SMTP end marker on the wire; the stored canonical raw
message remains unchanged.

STARTTLS is opportunistic by default and required when the TLS policy is
`Required`. If a server advertises STARTTLS, a failed handshake or certificate
validation is a delivery failure and is never silently downgraded. Certificates
are validated against WebPKI roots for the MX hostname. SMTP AUTH and remote
credentials are intentionally not implemented.

Remote `2xx` acceptance means only that the next SMTP server accepted the
message. It does not mean inbox placement. Temporary and permanent SMTP
responses are persisted independently per recipient; ambiguous post-DATA
outcomes are marked uncertain and are not automatically retried. MX fallback
is limited to pre-submission connection/protocol failures. This bounded profile
does not yet claim production readiness: tenant-specific TLS policy, IP-level
throttling, circuit breakers, MTA-STS/DANE, delivery telemetry, and live
interoperability evidence remain outstanding.
