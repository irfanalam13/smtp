# Phase 87 — Search and Global Indexing

Status: **Mailbox-scoped PostgreSQL search foundation exists; tenant-wide SaaS search pending**

## Repository baseline

PostgreSQL migrations add mailbox search projections and indexes, and a
bounded mailbox-scoped search function enforces tenant/mailbox predicates.
Projection population, authenticated search API, and relevance/freshness SLO
evidence remain incomplete. There is no global index across users,
organizations, templates, logs, or analytics, and no OpenSearch/Elasticsearch
deployment.

## Recommended search boundary

Complete and measure PostgreSQL mailbox search and its projection worker
before adding a dedicated search cluster. Define per-resource authorization
and document freshness separately for messages, identities, templates, and
operational logs. Never make a global index the authorization source; enforce
current tenant/resource permissions when querying and before returning hits.

If measured volume requires OpenSearch, index only minimized, tenant-tagged,
rebuildable projections; exclude raw bodies/attachments unless explicitly
needed and privacy-reviewed. Secure cluster access, tenant-aware routing,
deletion propagation, replay/rebuild, index lifecycle, and disaster recovery.
Search snippets/highlights must be treated as untrusted content.

## Remaining work and why

First populate and benchmark existing mailbox indexes; define corpus, tenant
skew, query relevance, freshness, privacy, retention, and SLOs. Only then
evaluate PostgreSQL versus a separate engine, cost, cluster operations,
residency, deletion, and failure recovery. Global users/templates/logs search
needs separate resource-specific query and authorization contracts.

This remains open because current search covers a bounded mailbox query, not
enterprise-wide indexing, and projections are not yet reliably populated.
Adding a search cluster before measuring the PostgreSQL baseline would add
another sensitive data copy and authorization/deletion path without evidence.

## Acceptance criteria

- Every indexed field has an owner, minimization purpose, retention, and
  deletion/rebuild policy.
- Search results are filtered by current authorization, including nested
  resources and guessed IDs.
- Projection lag, replay, deletion, and rebuild are measured and observable.
- Query complexity, payload, fan-out, and tenant resource budgets are bounded.
- A dedicated engine is justified by benchmark evidence and passes restore,
  tenant-isolation, and deletion tests.

## Research references

- [Phase 48 Mail Indexing](phase-48-mail-indexing.md)
- [Phase 49 Full-Text Search](phase-49-full-text-search.md)
- [Phase 50 Search Optimization](phase-50-search-optimization.md)
- [OpenSearch documentation](https://docs.opensearch.org/latest/)
