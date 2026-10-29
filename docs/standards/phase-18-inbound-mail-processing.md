# Phase 18 — Inbound Mail Processing

Status: **Durable inbound acceptance foundation exists; routing profile
continues**

The inbound acceptor validates bounded recipients, resolves only active
mailboxes on verified domains, stores raw message bytes and recipient metadata
transactionally, and returns SMTP success only after durable commit.

Remaining production work includes multi-recipient routing semantics,
tenant/domain policy, aliases and forwarding, quota enforcement, duplicate
delivery policy, retention, malware scanning integration, and mailbox indexing.
Raw message storage must remain separate from parsed display fields.
