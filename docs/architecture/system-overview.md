# System overview

SMTP protocol services run in Rust under `smtp-session/`. The `smtp-ingress` crate
is a loopback-only, bounded SMTP server by default. Without a database queue or
inbound acceptor it operates as a protocol test server; with those explicit
dependencies configured it can durably enqueue outbound messages or accept
inbound mail.

The Go control plane is in `smtp-api`, shared Rust contracts are in
`smtp-core`, and the Next.js dashboard is in `smtp-dashboard`. PostgreSQL
migrations and storage ownership are in `smtp-storage`. PostgreSQL is the
initial system of record and is started locally with Compose.

The ingress server defaults to `127.0.0.1:2525`. It supports `EHLO`, `HELO`,
`MAIL FROM`, `RCPT TO`, `DATA`, `RSET`, `NOOP`, and `QUIT`. `STARTTLS` and
`AUTH` are explicitly rejected with `502`.

When `DATABASE_URL` is set, the runnable ingress connects to PostgreSQL for
the durable queue. Inbound persistence is a separate opt-in
`InboundAcceptor`; it returns `250` from `DATA` only after routing metadata and
the raw message commit. Without these explicit integrations, the library
remains in its bounded protocol-test mode.

The outbound `mail-delivery` crate in `smtp-delivery` is a separate queue-worker
processor.
Delivery is disabled by default. When explicitly enabled, it resolves MX
records, filters unsafe destinations, connects to the validated IP, and uses a
bounded SMTP dialogue with opportunistic or required STARTTLS. A remote `2xx`
response means remote SMTP acceptance only; it is not mailbox placement.

The session transitions from `Connected` to `Greeted`, then through
`MailFromAccepted` and `RecipientsAccepted`; `DATA` returns to `Greeted` after
the configured acceptance boundary succeeds. `RSET` also returns to `Greeted`,
while `QUIT` closes the session. Defaults are 64 connections, 30 seconds per
command/data line, 1,000 bytes per line, 100 recipients, 10 MiB per message,
and 64 KiB buffered input per connection.
