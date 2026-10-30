# Phase 151 — AI Search and Semantic Retrieval

Status: **Bounded exact cosine-ranking primitive added; embeddings, durable index, and authorization integration pending**

## Repository baseline and code slice

The shared Rust crate now provides `semantic_search` over caller-supplied
embeddings. It validates dimensions and finite/non-zero vectors, bounds
candidate/result/scope sizes, ranks by cosine similarity deterministically,
and returns mailbox/message IDs plus scores only. It filters candidates to an
explicit tenant and mailbox scope before examining their vectors.

The caller must obtain that scope from authoritative authorization and ensure
the candidate set itself was loaded safely. This in-memory primitive does not
generate embeddings, read mail, provide approximate nearest-neighbor indexing,
or create a complete authorization boundary.

## Recommended design

Version embedding models and vector schemas; isolate indexes by tenant or
enforce tenant/resource filters inside the storage query, before ranking.
Authorize message and attachment access at query time and again when
hydrating snippets. Minimize embeddings and define retention/deletion for
vectors when source mail changes or is deleted. Natural-language queries may
contain sensitive data and must follow the same privacy policy as message
content. Apply bounded top-k, time, dimension, and cost limits.

## Remaining work and why

Select an embedding model, vector store, lifecycle strategy, search
authorization adapter, and evaluated retrieval dataset. Add durable indexing,
incremental updates, permission-change invalidation, and measured retrieval
quality. This remains open because there is no model, vector database,
tenant-authenticated search service, or production workload.

## Acceptance criteria

- Authorization filters are enforced before content/vector hydration and are
  tested across tenants and mailbox permissions.
- Embedding model/version and source-message provenance are retained.
- Deleted, moved, or newly restricted messages are promptly removed or
  excluded from retrieval.
- Retrieval quality, latency, cost, and cross-tenant isolation are evaluated.
- The result API returns only authorized identifiers/scores; snippets require
  a separate authenticated read.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [Phase 87 Search and Global Indexing](phase-87-search-global-indexing.md)
