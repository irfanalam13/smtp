# Phase 111 — Scalability Architecture and Capacity Model

Status: **Design and repository assessment; production capacity work pending**

## Goal and scope

Establish measurable capacity objectives and a staged scale-out architecture
for API, SMTP ingress, delivery workers, mailbox services, databases, and
object storage. This phase frames the five scalability work items in Phases
111–115; it does not assert a target mailbox count or throughput without
representative load evidence.

## Repository baseline

The API is a single Go process backed by PostgreSQL. The repository has
process-local DNS/rate-limit state and process-local metrics. Phase 98 already
records that there is no production deployment, autoscaling configuration,
distributed cache, sharding, or validated multi-region failover. Phases 71–110
also remain incomplete, including tenant isolation, durable production
services, and clustered HA.

## Recommended design

Set service-level indicators and objectives first: accepted SMTP messages,
delivery latency, queue age, API latency/error rate, mailbox operation
latency, storage durability, and per-tenant fairness. Define realistic
concurrent load profiles, message-size distributions, recipient fan-out,
provider throttling, tenant skew, and dependency failures. Measure CPU,
memory, network, database connection/lock pressure, queue lag, object-store
latency, and cost at each tested load.

Scale stateless request handlers independently from stateful storage and
workers. Maintain explicit connection and concurrency budgets so increasing
replicas cannot exceed PostgreSQL, DNS, remote MX, or object-store limits.
Preserve tenant isolation, idempotency, queue leases, and mailbox consistency
across replicas. Adopt caching, sharding, and multi-region writes only through
separately reviewed designs with measured need and rollback plans.

## Remaining work and why

Approve SLOs, workload profiles, deployment topology, and capacity ownership;
then benchmark representative workloads and identify bottlenecks before
selecting scaling mechanisms. "Millions of mailboxes" is not a measurable
capacity claim by itself. A source change cannot prove sustainable throughput,
isolation, or recovery without production-like infrastructure and repeatable
load evidence.

## Acceptance criteria

- Each critical user flow has approved SLI/SLO, error budget, and load model.
- Reproducible tests quantify saturation and tail latency across expected
  tenant skew, bursts, large messages, retries, and dependency degradation.
- Replica and connection budgets are derived from measured downstream limits.
- Scale-out and scale-in preserve message durability, idempotency, and fairness.
- Results, test environment, build revision, and known limitations are recorded.

## Research references

- [Phase 98 Enterprise Scalability](phase-98-enterprise-scalability.md)
- [Google SRE Workbook](https://sre.google/workbook/table-of-contents/)
- [Kubernetes Horizontal Pod Autoscaling](https://kubernetes.io/docs/concepts/workloads/autoscaling/horizontal-pod-autoscale/)
