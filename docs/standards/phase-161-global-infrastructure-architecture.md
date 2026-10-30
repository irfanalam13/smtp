# Phase 161 — Global Infrastructure Architecture

Status: **Architecture plan; no production region or global topology deployed**

## Current development scope

The immediate target is a single Ubuntu laptop with services bound to
loopback. No cloud account, paid hosting, public DNS, or Kubernetes cluster is
required to develop and run the current application. Treat Phases 161–180 as
future hosted-platform designs; validate local behavior first and do not
represent laptop processes as multi-region infrastructure.

## Repository baseline

AWS EKS appears in earlier design notes as a possible future target, not a
current decision or provisioned environment. There is no AWS account, region,
VPC, cluster, production DNS, or operational ownership configured. Cloud
deployment decisions are deferred until there is an approved need and budget.

## Recommended design

Inventory each service by statefulness, authoritative data, latency
sensitivity, tenant residency, mail reputation/egress, failure domain, and
recovery objective. Begin with active/passive regional writes and stateless
regional replicas; do not assume globally writable mail data. Separate public
SMTP ingress, customer API/web traffic, internal service traffic, and
operations networks. Map dependencies, keys, DNS, identity, observability,
backups, and support operations for each region.

## Remaining work and why

Approve customer/geographic demand, data residency, SLO/RTO/RPO, costs,
regions, service topology, and operating ownership. This remains open because
global topology changes legal boundaries, mail delivery identity, consistency,
and recovery behavior; no deployment facts exist to choose safely.

## Acceptance criteria

- Every service/data class has an owner, home region, replication policy, and
  tested recovery contract.
- Diagrams show ingress/egress, trust boundaries, dependencies, and failure
  domains.
- Single-writer versus multi-writer semantics are explicit for mail, queue,
  identity, and audit data.
- Data residency and global support access have documented controls.
- Cost, regional service availability, and SMTP egress constraints are
  validated before implementation.

## Research references

- [Repository deployment topology](../architecture/phase-01/data-and-deployment.md)
- [Amazon EKS Best Practices](https://docs.aws.amazon.com/eks/latest/best-practices/introduction.html)
- [Phase 122 Cross-Region Replication](phase-122-cross-region-replication.md)
