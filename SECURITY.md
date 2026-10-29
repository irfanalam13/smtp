# Security policy

## Current status

SMTP Platform is under active development. The repository contains local
protocol and control-plane prototypes and must not be exposed as a production
mail service without the security work documented in
[Phase 01](docs/architecture/phase-01/security-architecture.md) and
[Phase 02](docs/standards/security-requirements.md).

## Reporting a vulnerability

The project does not yet have an established private security mailbox,
security advisory URL, or bug-bounty program. Do not invent or guess a
reporting address. Until a private channel is established, do not disclose
potentially exploitable details in a public issue; contact the repository
maintainers through the hosting platform's private security-reporting
mechanism, if enabled, and record the missing channel as a release blocker.

Include a concise description, affected component, reproduction steps that do
not use real credentials or customer data, impact assessment, and proposed
mitigation. Redact secrets and message content.

## Development security rules

- Never commit `.env`, credentials, private keys, tokens, or production data.
- Keep local database ports bound to localhost.
- Keep external SMTP delivery disabled unless explicitly configured for a
  controlled fixture.
- Treat SMTP, DNS, message, and webhook input as untrusted.
- Do not disable TLS certificate validation as a general workaround.
- Report dependency or supply-chain concerns with the affected version and
  lockfile context.
