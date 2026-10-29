# Governance, observability, risks, and phase dependencies

## Observability architecture

Use structured logs, metrics, traces, health endpoints, and restricted audit records. Correlation IDs connect API request, SMTP transaction, message, queue lease, delivery attempt, and event, but tenant-safe identifiers are preferred over raw addresses.

Critical signals: acceptance/rejection and p95 latency; queue depth and oldest age; attempts, delivery/deferral/permanent-failure rates; destination SMTP codes; retry storms; worker utilization; DB latency/errors; content-store errors; event lag; webhook failures; auth failures; abuse detections; resource saturation. Alert on symptoms and burn rates, not every transient SMTP 4xx.

Telemetry must remain useful during application outage via an independent collector/sink path. Bodies, credentials, tokens, full auth headers, and unnecessary personal data are redacted. Audit events are separate from debug logs and have stricter access/retention.

## Licensing and open-source recommendation

Retain the existing Apache-2.0 repository direction provisionally: it is permissive for hosted and redistributed use and matches the current Rust workspace metadata. A future governance decision must confirm whether SDKs/docs use the same license, whether copyleft dependencies are acceptable, trademark ownership, contributor license expectations, security disclosure, and commercial hosted-service terms. Dependency license compatibility and notices are required; open source does not remove redistribution obligations. This is an engineering recommendation, not legal advice.

## Risk register

| ID    | Severity | Risk                                                            | Mitigation/validation                                                                       | Owner                | Phase |
| ----- | -------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | -------------------- | ----- |
| R-001 | Critical | Acknowledged message loss at content/queue boundary             | Transactional acceptance record, outbox, reconciliation, crash tests                        | Data-plane lead      | 06/10 |
| R-002 | High     | Duplicate delivery after ambiguous SMTP outcome                 | Explicit ambiguous state, bounded retries, idempotent local transitions, customer semantics | Delivery lead        | 02/10 |
| R-003 | High     | Open relay or tenant escape                                     | Authz matrix, parser tests, negative cross-tenant tests, external review                    | Security lead        | 07    |
| R-004 | High     | Reputation damage from abuse/credential compromise              | Quotas, anomaly detection, suspension, DKIM/DMARC, operator runbook                         | Abuse lead           | 07/10 |
| R-005 | High     | Queue or destination retry storm                                | Fair scheduling, budgets, circuit breakers, load tests                                      | Delivery lead        | 10    |
| R-006 | High     | Control-plane in-memory state used in production                | PostgreSQL repositories and deployment guardrail                                            | Control-plane lead   | 06/08 |
| R-007 | Medium   | Database becomes shared bottleneck                              | Owner roles, pools, partitions/read models, benchmarks                                      | Data lead            | 06    |
| R-008 | Medium   | Over-decomposition increases failure modes                      | Stable modular boundaries and split criteria                                                | Architecture owner   | 03/05 |
| R-009 | Medium   | Cloud blocks outbound SMTP or lacks stable egress               | Provider preflight and test account before commitment                                       | Platform lead        | 05    |
| R-010 | Medium   | Retention/data residency obligations unclear                    | Data inventory, legal review, deletion/restore tests                                        | Product/security     | 00/06 |
| R-011 | Medium   | Backup restore is untested                                      | Scheduled encrypted restore drills and reconciliation                                       | SRE                  | 05/10 |
| R-012 | Medium   | Hosted mailbox scope causes premature search/storage complexity | Keep P1/P2 boundary; separate mailbox ADR                                                   | Product/architecture | 00/06 |

## Phase dependency matrix

| Phase                  | Phase 01 output                                                                          |
| ---------------------- | ---------------------------------------------------------------------------------------- |
| 00 Vision              | Confirm transactional delivery scope, users, success criteria, regions, retention, cost  |
| 02 RFC research        | SMTP state/extension ownership, TLS, SPF/DKIM/DMARC, bounce and compliance requirements  |
| 03 Development         | Rust/Go workspace conventions, local services, protocol test fixtures, config contracts  |
| 04 Core infrastructure | Redaction, error taxonomy, config snapshots, service identity, event abstractions        |
| 05 DevOps              | images, SBOM, signing, environment isolation, deployment target, IaC, restore automation |
| 06 Data/storage        | authoritative schemas, outbox/content protocol, retention, partitioning, backup/restore  |
| 07 Security            | OIDC, SMTP AUTH, RBAC, KMS, abuse controls, threat tests, audit                          |
| 08 API/communication   | OpenAPI, idempotency, versioning, event envelope, webhooks, deprecation                  |
| 09 Observability       | instrumentation, dashboards, alerts, SLO measurement, independent telemetry              |
| 10 Validation          | crash/duplicate/load/tenant-isolation/security/DR acceptance tests                       |

## Open questions

1. Which initial cloud/provider and regions meet outbound SMTP, stable egress, reverse-DNS, and data-residency needs?
2. What are the product's contractual volume, retention, RTO/RPO, and compliance requirements?
3. Which OIDC issuer and audience model will be supported?
4. Is inbound mail a launch requirement or a later product?
5. What malware scanning, complaint, reputation, and billing integrations are required?
6. What message-size/content retention and deletion/legal-hold policy is approved?

## Deferred work

No SMTP implementation, API endpoint, migration, production deployment, Kubernetes manifest, dashboard, billing integration, full RFC compliance, or production secret was added by Phase 01.
