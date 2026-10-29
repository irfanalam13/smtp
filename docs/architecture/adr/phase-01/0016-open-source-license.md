# ADR-0016: Apache-2.0 licensing direction

- Status: Proposed
- Date: 2026-10-28

## Context

The Rust workspace already declares Apache-2.0, while the platform needs broad contribution and hosted-service flexibility. Dependency, SDK, documentation, trademark, and governance implications are not yet fully reviewed.

## Decision

Continue with Apache-2.0 as the provisional repository recommendation. Require dependency license reports, notices, contribution/security policies, and professional legal review before a public release or license change.

## Consequences

Permissive reuse is straightforward, but the project must track notices, patents/contributor policy, trademarks, and any copyleft dependencies. This decision can be revised before the first stable release without changing the runtime architecture.
