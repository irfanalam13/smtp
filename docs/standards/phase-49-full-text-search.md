# Phase 49 — Full-Text Search

Status: **Tenant-scoped PostgreSQL search foundation implemented; complete query grammar and production search service pending**

## Query contract

The current durable endpoint function uses PostgreSQL
`websearch_to_tsquery('simple', ...)` and weighted `ts_rank_cd` over subject,
sender, recipients, body text, and attachment names. It supports web-search
style terms, quoted phrases, `OR`, and minus-prefixed exclusions. Optional
fuzzy matching uses the `pg_trgm` similarity operator for sender and subject.
The SQL constrains both tenant ID and mailbox ID before returning results.

The API limits query length to 512 characters, page size, and minimum fuzzy query length. Search
is a relevance tool, not an authorization system; stale/deleted messages are
excluded and the caller must still enforce current access. The simple
configuration is intentionally not stemming or language-specific; language
profiles need an explicit product decision.

## Implemented code and gaps

`search_mailbox` queries PostgreSQL full-text indexes, supports ranked
pagination and optional fuzzy sender/subject matching, and returns mailbox
UIDs and metadata. Integration coverage verifies phrase matches and
cross-tenant isolation. The existing in-memory `SearchIndex` remains a
testable reference only.

Still required: query operator contract/versioning, fielded `from:`, `to:`,
`subject:`, `before:` and `after:` filters, safe highlighting, stable
cursor-based pagination, search endpoint/auth integration, parser-to-index
population, cancellation/timeouts, search abuse controls, multilingual
configuration, and representative relevance testing. “Enterprise-grade” and
“instant” need measured SLOs and workload evidence before those terms can be
claimed.

## Acceptance criteria

- Search is isolated across tenants, folders, and current mailbox grants.
- Phrase, boolean, exclusion, fuzzy, empty, malformed, and adversarial queries
  have deterministic behavior.
- Ranking is evaluated against an approved judged-query set.
- p95/p99 latency and freshness lag meet explicit product SLOs at target
  corpus size and concurrency.
