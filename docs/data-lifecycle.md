# Data lifecycle and retention

## Data classes

| Data | Current storage | Current exposure |
| --- | --- | --- |
| Tenant, domain, membership, and API-key metadata | In-memory Go store; schema migration exists | Tenant-scoped API responses |
| API-key secret | Never stored in plaintext; hash in the development store | Returned once at creation |
| Queue message and envelope data | PostgreSQL queue tables when queue integration is enabled | Worker and queue diagnostics |
| Inbound raw message and routing metadata | PostgreSQL inbound tables when enabled | Metadata APIs only |
| DNS check results | In-memory control-plane cache | Tenant-scoped readiness APIs |
| Logs and metrics | Process output/metrics hooks | Must exclude message content and credentials |

## Current behavior

The repository does not yet run retention jobs. Queue rows, inbound messages,
attempt history, and raw message bytes therefore remain until an operator
removes them according to an approved local-data policy. The development
dashboard does not render raw email HTML.

## Production requirements

Before production use, define and implement:

- Queue and inbound-message retention periods.
- Legal hold and deletion behavior.
- Raw-message and attachment storage boundaries.
- Tenant export and deletion workflows.
- Backup encryption and restore testing.
- Database access audit and least-privilege roles.
- TTL and purge monitoring.

Do not treat PostgreSQL persistence as an indefinite retention policy.
