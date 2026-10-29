# Outbound SMTP delivery

`smtp-delivery` is the controlled outbound transport for queued
messages. It is not called by the SMTP ingress directly; a queue worker invokes
the `SmtpProcessor`, leaving claims, leases, retries, and terminal state
transitions owned by `mail-queue`.

Delivery is disabled unless `SMTP_DELIVERY_ENABLED=true` is explicitly
configured. Local fixture delivery additionally requires an explicit loopback
endpoint and `allow_loopback_test_destinations`. Production delivery resolves
MX records, sorts candidates by preference, resolves each MX host to address
records, filters unsafe addresses, and connects to the validated address
directly. The client does not perform a second hostname-based connection after
the safety check.

The SMTP dialogue is bounded at every stage: connection, commands, replies,
message bytes, MX candidates, and fallback attempts. Replies are parsed as
bounded single- or multi-line SMTP responses. Message bytes are normalized to
CRLF, dot-stuffed, and terminated with the SMTP end marker.

STARTTLS is opportunistic by default and required when the TLS policy is
`Required`. If a server advertises STARTTLS, a failed handshake or certificate
validation is a delivery failure and is never silently downgraded. Certificates
are validated against WebPKI roots for the MX hostname. SMTP AUTH and remote
credentials are intentionally not implemented.

Remote `2xx` acceptance means only that the next SMTP server accepted the
message. It does not mean inbox placement. Temporary and permanent SMTP
responses are classified separately; ambiguous final responses are never
reported as success. The current queue schema records aggregate attempt
outcomes, while per-recipient acceptance and rejection counts remain in the
transport report and are not written to message bodies or logs.
