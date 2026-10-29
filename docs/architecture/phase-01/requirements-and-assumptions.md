# Requirements and assumptions

## Functional requirements

| ID     | Priority | Requirement                                                                    | Owner                  | Dependencies                        | Acceptance criterion                                                            | Target                   |
| ------ | -------- | ------------------------------------------------------------------------------ | ---------------------- | ----------------------------------- | ------------------------------------------------------------------------------- | ------------------------ |
| FR-001 | P0       | Authenticate API and SMTP submitters and authorize tenant-scoped actions       | Identity/control plane | OIDC decision, API-key policy       | Every mutation has an authenticated principal and tenant authorization decision | Phase 07/08              |
| FR-002 | P0       | Register and verify sending domains and identities                             | Domain/control plane   | DNS resolver, audit                 | Unverified identities cannot submit                                             | Phase 08                 |
| FR-003 | P0       | Accept a message only after durable responsibility is established              | Submission/queue       | PostgreSQL, content storage         | Crash/restart recovery finds every acknowledged message                         | Phase 06/10              |
| FR-004 | P0       | Schedule, retry, and deliver via SMTP                                          | Queue/delivery         | DNS, TLS, policy                    | Temporary failures retry with bounded backoff; permanent failures terminate     | Phase 02/10              |
| FR-005 | P0       | Expose customer-safe delivery events and status                                | Events/API             | Event envelope, idempotency         | Consumers can replay without duplicate customer effects                         | Phase 08/09              |
| FR-006 | P0       | Enforce quotas, rate limits, suppression, and emergency suspension             | Policy/abuse           | Durable counters, reputation inputs | A tenant cannot bypass limits by changing API workers                           | Phase 07/10              |
| FR-007 | P0       | Prevent open relay, injection, and unsafe destination access                   | SMTP/security          | Parser, network controls            | Negative security tests reject unauthenticated relay and unsafe targets         | Phase 02/07              |
| FR-008 | P0       | Provide audit and operational telemetry without message-content leakage        | Audit/observability    | Redaction, retention                | Investigators can correlate a message without reading its body in logs          | Phase 09                 |
| FR-009 | P1       | Process inbound mail for explicitly provisioned mailboxes                      | Inbound/mailbox        | Verified domains, storage           | Only active mailbox recipients are accepted after durable commit                | Later milestone          |
| FR-010 | P1       | Provide bounce classification and suppression updates                          | Bounce/policy          | Delivery events, feedback           | Repeated hard failures prevent future sends according to policy                 | Phase 08/10              |
| FR-011 | P1       | Provide mailbox access and search                                              | Mailbox                | Object storage, index               | IMAP/POP3 or API access follows mailbox authorization and retention             | Future hosted-mail phase |
| FR-012 | P2       | Webmail, multi-region active-active delivery, and advanced reputation products | Product extensions     | Business/compliance decisions       | Separate RFC and capacity validation exists before commitment                   | Future                   |

## Non-functional requirements

These are **provisional targets**, not service-level guarantees. They must be validated with production-like load and failure tests.

| Area                | Initial target                                                                                                                   | Assumption and validation                                                                   |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Availability        | 99.9% monthly for API and submission in one region; delivery continues through control-plane outages for the policy lease window | Validate with AZ and dependency failure tests; target cost rises materially at multi-region |
| Durable acceptance  | No acknowledged message lost after a committed transaction; RPO 0 within the configured PostgreSQL durability boundary           | Validate crash injection around commit/outbox/content writes                                |
| Delivery processing | At-least-once internal processing; no universal exactly-once remote delivery claim                                               | Validate duplicate/ambiguous SMTP outcomes                                                  |
| Latency             | API p95 < 300 ms excluding downstream delivery; accepted SMTP transaction p95 < 1 s when storage is healthy                      | Benchmark with representative message sizes                                                 |
| Recovery            | RTO 1 hour for a regional single-system restore; RPO 15 minutes for analytics, 0 for acknowledged message metadata               | Confirm backup restore and operator runbooks                                                |
| Isolation           | Per-tenant authorization on every control-plane operation; fair queue scheduling and bounded per-tenant concurrency              | Security and noisy-neighbor tests                                                           |
| Retention           | Configurable message retention; audit and billing retention are separately governed                                              | Legal/compliance review required                                                            |
| Security            | TLS at external boundaries, authenticated internal service calls, encrypted data at rest, key rotation, abuse controls           | Threat model and penetration testing                                                        |

## Provisional workload baseline

The repository has no production measurements. Capacity work therefore starts with a recalculable baseline, not a promise:

- 100 tenants, 1,000 verified domains, and no hosted users in the first public deployment.
- 10 messages/sec sustained, 100 messages/sec burst, 256 KiB median raw message, 25 MiB maximum configurable message.
- 5% attachment incidence in the initial delivery product; attachments are part of raw message content.
- 24-hour queue retention during downstream outage, with operator-configurable maximum.
- One region, two availability zones, stable egress IPs, and a single PostgreSQL primary with synchronous standby where affordable.

Capacity formulae:

```text
ingress_bytes_per_second = messages_per_second * average_message_bytes
queue_storage = ingress_bytes_per_second * outage_seconds * replication_factor
worker_count >= peak_messages_per_second * mean_delivery_seconds / target_worker_utilization
event_rate = accepted_messages_per_second * events_per_message
```

Benchmark inputs must replace each estimate before increasing limits. SMTP destination latency and remote throttling are expected to dominate worker capacity.

## Workload isolation

SMTP ingress and outbound workers use separate pools, process limits, database connection budgets, and queue partitions. Control-plane API, analytics, webhooks, and dashboard workloads may share PostgreSQL initially only with separate roles, statement timeouts, and connection pools. Object-storage transfers are bounded and never performed on a transaction holding a queue lease. Search is not a P0 dependency.
