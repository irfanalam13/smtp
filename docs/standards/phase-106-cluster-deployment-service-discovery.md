# Phase 106 — Cluster Deployment and Service Discovery

Status: **AWS EKS selected; no production cluster, manifests, or service discovery implemented**

## Repository baseline

The development topology uses local processes and Compose PostgreSQL.
Architecture documents describe a future multi-instance deployment but do not
implement an AWS EKS cluster, AWS load balancer, IRSA/EKS Pod Identity, or
cluster operator. AWS EKS is the selected target; service sizing, networking,
ingress/egress, AWS account ownership, and cluster management remain to be
specified.

## Recommended design

Use AWS EKS with stateless replicas behind an AWS load balancer and Kubernetes
service discovery. Define readiness separately from liveness; readiness checks
dependencies needed to accept work, while liveness must not cause cascading
restarts during dependency outages. Add connection draining, bounded startup
and shutdown, zone placement, capacity limits, network policy, and workload
identity. Do not put SMTP protocol state or durable mail state in a local
container filesystem.

## Remaining work and why

Define EKS cluster/network topology, failure domains, SLO/RTO/RPO, ingress and
SMTP egress design, load balancing, service naming, IAM, secrets, and
observability. Package and deploy each service independently; test zone loss,
DNS changes, drains, and deployment rollback. This remains open because no
AWS account or operator model exists and the current service set is not all
production deployable.

## Acceptance criteria

- Services run in at least two independent failure domains with tested
  balancing and connection draining.
- Readiness/liveness semantics do not create retry/restart storms.
- Service discovery, identity, network policy, secrets, and egress are
  documented and automated.
- Deploy/rollback preserves queue and database compatibility.
- Provider failure and zone loss tests meet approved service objectives.

## Research references

- [Kubernetes Nodes](https://kubernetes.io/docs/concepts/architecture/nodes/)
- [Deployment topology](../architecture/phase-01/data-and-deployment.md)
- [Phase 98 Enterprise Scalability](phase-98-enterprise-scalability.md)
