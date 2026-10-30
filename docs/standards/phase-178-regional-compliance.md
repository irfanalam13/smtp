# Phase 178 — Regional Compliance and Data Residency

Status: **No region placement or jurisdictional controls implemented**

## Recommended design

Map each data class and processing activity to allowed regions, including
replicas, backups, logs, support access, model inference, DNS metadata, and
subprocessors. Enforce placement at provisioning and service boundaries;
network proximity is not residency. Define migration, deletion, legal hold,
and disaster-recovery exceptions with legal and customer review.

## Remaining work and why

Complete data inventory, jurisdiction analysis, customer commitments,
subprocessor review, and technical placement controls. This remains open
because actual customer, contractual, and regional facts are not established.

## Acceptance criteria

- Region eligibility is explicit per tenant and data class.
- Residency is verified for primary, replica, backup, telemetry, support, and
  AI processing paths.
- Cross-region failover cannot silently violate tenant policy.
- Data movement and administrative access are audited.
- Legal/compliance owners approve applicable claims and exceptions.

## Research references

- [Phase 126 Compliance Governance](phase-126-compliance-scope-and-governance.md)
- [Phase 122 Cross-Region Replication](phase-122-cross-region-replication.md)
- [AWS S3 replication](https://docs.aws.amazon.com/AmazonS3/latest/userguide/replication.html)
