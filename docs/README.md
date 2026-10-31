# Documentation

## Start here

- [Development setup](development/setup.md) — prerequisites, first-time
  bootstrap, normal startup, migrations, tests, and shutdown.
- [Core infrastructure](development/core-infrastructure.md) — shared
  configuration, secrets, logging, context, events, and shutdown contracts.
- [Project status](project-status.md) — what Milestones 0–8 implement and what
  remains before production.
- [Backend platform audit](backend-platform-audit.md) — M00 runtime wiring,
  feature classifications, prioritized gaps, and baseline check results.
- [Transactional email product scope](product/transactional-email-platform.md)
  — market references, customer page map, implementation sequence, and
  capability release gates.
- [Phase implementation status](phase-status.md) — remaining code by phase,
  deferral reasons, and when each item should be applied.
- [Phase 01–10 integration review](phase-integration.md) — phase handoffs,
  connected paths, integration gaps, and the completion boundary.
- [API documentation](api/README.md) — REST conventions, contract, and
  endpoint development guidance.
- [Observability](observability/README.md) — logging, metrics, health checks,
  dashboards, alerts, and deferred telemetry infrastructure.
- [Phase 10 validation](validation/README.md) — evidence matrix, performance
  baseline, risk register, and release-readiness assessment.
- [Operations guide](operations.md) — local service map, logs, health checks,
  failure recovery, and safe database handling.
- [Configuration reference](configuration.md) — environment variables,
  ownership, and safety rules.
- [Testing guide](testing.md) — standard checks, focused tests, and current
  coverage gaps.
- [Data lifecycle](data-lifecycle.md) — stored data, exposure boundaries,
  retention gaps, and production requirements.
- [Mailbox platform phases 41–45](standards/phase-41-mailbox-architecture.md)
  — architecture, mailbox ownership, folders, message storage, and metadata.
- [Attachment and search phases 46–50](standards/phase-46-attachment-storage.md)
  — secure attachment storage, MIME parsing, indexing, search, and tuning.
- [Conversation and mailbox workflow phases 51–60](standards/phase-51-conversation-threading.md)
  — threading, bounded filters, labels, quarantine, retention, drafts, and
  sent-mail tracking.

## Architecture

- [Repository structure and feature ownership](architecture/repository-structure.md)
- [Phase 01 system architecture](architecture/phase-01/)
- [Phase 02 standards and RFC research](standards/)
- [DevOps and CI/CD](devops/)
- [Database and storage](storage/)
- [System overview](architecture/system-overview.md)
- [Durable mail queue](architecture/durable-mail-queue.md)
- [Outbound delivery](architecture/outbound-delivery.md)
- [Inbound mail](architecture/inbound-mail.md)
- [Domain readiness](architecture/domain-readiness.md)
- [Architecture decision records](architecture/adr/)

## API and security

- [OpenAPI contract](api/openapi.yaml)
- [Security baseline](security/baseline.md)
- [Security architecture](security/security-architecture.md)
- [Threat model](security/threat-model.md)
- [Authentication and authorization](security/authentication-authorization.md)
- [Cryptography and TLS](security/cryptography-and-tls.md)
- [Secrets and key management](security/secrets-and-key-management.md)
- [Audit logging](security/audit-logging.md)
- [Security testing](security/security-testing.md)
- [Incident response](security/incident-response.md)

Documentation must describe implemented behavior honestly. In particular,
development authentication, in-memory control-plane state, advisory DNS
readiness checks, disabled-by-default external delivery, and the limited
secret-mounted Ed25519 signing profile must not be presented as complete
production capabilities.
