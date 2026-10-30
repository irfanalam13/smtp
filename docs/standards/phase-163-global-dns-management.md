# Phase 163 — Global DNS Management

Status: **No production DNS zones, health checks, or routing policies configured**

## Recommended design

Manage authoritative zones and records through reviewed infrastructure as
code, with registrar access separated from routine operations. Define DNSSEC,
TTL, ownership, change audit, health-check semantics, and rollback. Use
latency/failover policies only for endpoints whose regional behavior is
correct. DNS caches and resolver behavior mean DNS changes are not
instantaneous failover.

SMTP MX, SPF, DKIM, DMARC, MTA-STS, TLS reporting, and customer verification
records have mail-specific compatibility and reputation consequences; do not
route them through generic web endpoint policy without standards review.

## Remaining work and why

Select zone ownership, provider accounts, names, TTLs, health checks, DNSSEC
and change control. This remains open because domains and customer mail
identities are not provisioned in production and no DNS operations process
exists.

## Acceptance criteria

- DNS changes are reviewed, versioned, audited, and reversible.
- Health checks test service readiness and customer flows, not only TCP reachability.
- Failover behavior accounts for TTL, recursive caches, and stale answers.
- SMTP records and signing keys have independent validation and rollback.
- DNS provider and registrar access have least privilege and recovery contacts.

## Research references

- [Amazon Route 53 routing policies](https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/routing-policy.html)
- [Route 53 health checks and DNS failover](https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/dns-failover.html)
- [Phase 14 SMTP Security](phase-14-smtp-security.md)
