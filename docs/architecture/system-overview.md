# System overview

SMTP protocol services run in Rust under `smtp-session/`. The `smtp-ingress` crate
is a loopback-only, bounded SMTP server by default. Without a database queue or
queue integration it operates as a protocol test server. It can durably enqueue
outbound messages when a database queue is configured. A PostgreSQL inbound
acceptor exists, but the current executable does not wire it into the session.

The Go control plane is in `smtp-api`, shared Rust contracts are in
`smtp-core`, and the Next.js dashboard is in `smtp-dashboard`. PostgreSQL
migrations and storage ownership are in `smtp-storage`. PostgreSQL is the
initial system of record and is started locally with Compose.

The ingress server defaults to `127.0.0.1:2525`. It supports `EHLO`, `HELO`,
`MAIL FROM`, `RCPT TO`, `DATA`, `RSET`, `NOOP`, and `QUIT`. When certificate
material is configured, it supports STARTTLS; AUTH PLAIN can be enabled with a
PostgreSQL credential verifier and is advertised only after TLS. Credentials
are tenant-scoped Argon2id hashes. Non-loopback submission requires
authentication unless an inbound recipient-policy adapter is configured.
Credential provisioning is not yet exposed through an administrative API, and
authentication throttles remain process-local.

When `DATABASE_URL` is set, the runnable ingress connects to PostgreSQL for
the durable queue. An inbound `PostgresInboundStore` implementation exists,
but the current SMTP executable does not construct or inject it. Inbound
persistence is therefore not currently an end-to-end runtime path.

The `mail-delivery` crate in `smtp-delivery` contains an outbound SMTP client
and queue processor. Delivery is disabled by default, and the queue executable
currently uses a simulated test processor rather than this SMTP processor.
When the SMTP processor is explicitly wired and enabled, it resolves MX
records, filters unsafe destinations, connects to a validated IP, and uses a
bounded SMTP dialogue with opportunistic or required STARTTLS. A remote `2xx`
response means remote SMTP acceptance only; it is not mailbox placement.

The session transitions from `Connected` to `Greeted`, then through
`MailFromAccepted` and `RecipientsAccepted`; `DATA` returns to `Greeted` after
the configured acceptance boundary succeeds. `RSET` also returns to `Greeted`,
while `QUIT` closes the session. Defaults are 64 connections, 30 seconds per
command/data line, 1,000 bytes per line, 100 recipients, 10 MiB per message,
and 64 KiB buffered input per connection.
