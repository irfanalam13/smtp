# Data ownership and deployment

## Ownership matrix

| Data                                      | Authoritative owner      | Store                                         | Consistency/retention                                |
| ----------------------------------------- | ------------------------ | --------------------------------------------- | ---------------------------------------------------- |
| Tenants, users, roles                     | Control plane            | PostgreSQL                                    | Transactional; account/legal retention               |
| Credentials/API-key verifiers             | Identity/control plane   | PostgreSQL + secret/KMS systems               | Secret never recoverable; revoke/expiry              |
| Domains, DNS verification, identities     | Domain service           | PostgreSQL + bounded DNS cache                | Verification state transactional; DNS result expires |
| Message/envelope metadata and idempotency | Queue/data plane         | PostgreSQL                                    | Durable commit; message retention policy             |
| Raw RFC 5322 content/attachments          | Submission/mailbox owner | Encrypted object storage; bounded DB fallback | Content retention/deletion and legal hold            |
| Delivery attempts and outcomes            | Delivery/queue           | PostgreSQL                                    | Append attempt facts; operational retention          |
| Bounce/suppression                        | Bounce/policy            | PostgreSQL                                    | Policy-specific retention; customer export           |
| Customer-facing events                    | Event service            | Durable outbox/event store                    | Replayable bounded retention                         |
| Usage facts/aggregates                    | Analytics                | Derived tables/object store                   | Rebuildable from events where retained               |
| Audit events                              | Audit service            | Append-only restricted store                  | Longest governed retention                           |
| Mailbox metadata/indexes                  | Mailbox service (P1)     | PostgreSQL + optional search                  | Tenant/mailbox retention                             |

Detailed table schemas, indexes, migrations, and partition layouts belong to Phase 06. The critical Phase 01 boundary is that only the owner writes authoritative records.

## Deployment topology

### Development

The existing Compose/bootstrap flow runs PostgreSQL and local processes. SMTP binds loopback, delivery is disabled by default, development identity mode is explicit, and test databases are disposable. This is not a highly available topology.

### Minimal production

```text
Internet
  → managed edge/WAF/TLS
  → API/control instances (2+)
  → SMTP submission instances (2+; isolated pool)
  → queue/delivery workers (2+; destination bulkheads)
  → managed PostgreSQL (HA/backups)
  → encrypted object storage
  → secret manager/KMS
  → telemetry and audit sinks
```

Outbound requires stable egress IPs, valid reverse DNS, SPF/DKIM/DMARC alignment, provider approval for TCP/25 or submission ports, and destination reputation management. Cloud providers may restrict outbound SMTP; this must be verified before selecting a deployment target. Inbound requires MX/DNS, certificate, network, anti-abuse, and mailbox-routing design.

### Evolution

1. One region, managed PostgreSQL, isolated process pools, explicit limitations.
2. Multiple instances and availability zones; separate queue/delivery scaling.
3. Read projections, standby/restore drills, regional egress and tested failover.
4. Multi-region only after business RTO/data residency/reputation requirements and duplicate prevention are proven.

Containers are a packaging boundary, not a reason to require Kubernetes. Kubernetes, service mesh, and Terraform/OpenTofu modules are Phase 05 decisions after an actual provider and operating model are selected.

## Environment isolation

Development, test, staging, and production have separate accounts/projects, databases, keys, buckets, event subjects, domains, and egress identities. Promotion uses immutable artifacts, reviewed configuration, migration gates, rollback plans, and drift detection. Production secrets are never copied into lower environments.
