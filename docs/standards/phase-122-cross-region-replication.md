# Phase 122 — Cross-Region Replication

Status: **No production region or replication configuration selected**

## Goal and scope

Define replication for PostgreSQL, mail payloads, object data (if introduced),
keys, configuration, and operational state across failure domains. Replication
is not a backup: deletion, corruption, and compromised credentials may
replicate.

## Repository baseline

Local development uses a single PostgreSQL instance; object storage and
production managed services are not deployed. AWS EKS and KMS/Secrets Manager
are selected in planning but are not configured. There is no multi-region
topology, data residency policy, or failover evidence.

## Recommended design

Choose active/passive or active/active per data class and define consistency,
replication lag, conflict resolution, write fencing, regional independence,
encryption, data residency, and failback. Keep immutable/versioned backups
separate from replication. Ensure credentials and key-recovery procedures
remain available during a regional outage without weakening access controls.

## Remaining work and why

Approve regions, jurisdictions, target RPO/RTO, data classification, and
provider architecture. Implement replication only after storage and tenant
placement semantics are stable. This remains open because region and data
residency choices change legal and durability boundaries and require a real
deployment to validate.

## Acceptance criteria

- Replication lag and health are monitored and tied to approved recovery goals.
- Regional loss tests include database, object data, secrets, DNS, identity,
  queues, and customer ingress.
- A regional failover has single-writer/fencing protection and a documented
  failback procedure.
- Replication does not replace isolated, immutable backup recovery.
- Data residency and deletion behavior are reviewed for each data class.

## Research references

- [PostgreSQL high availability and replication](https://www.postgresql.org/docs/current/high-availability.html)
- [Phase 108 Stateful Storage High Availability](phase-108-stateful-storage-ha.md)
- [Phase 121 Backup Integrity](phase-121-backup-integrity-and-recovery-inventory.md)
