# Phase 160 — Unified AI Intelligence Platform

Status: **No integrated AI service; bounded offline contracts only**

## Repository baseline

Phases 131–150 contain design documents, an aggregate feature extractor, a
bounded review-only suggestion type, and an in-memory cosine-ranking
primitive. There are no AI models, provider integrations, persistent
embeddings, workflows, scanners, API surfaces, governance, or production
evaluation. These components do not establish an integrated platform.

## Recommended design

Compose independent capabilities behind versioned interfaces and a shared
policy gateway. Preserve source-of-truth ownership in mail services; AI
services provide advisory scores/suggestions, while deterministic
authorization and business rules control all side effects. Share only
approved, minimal features; enforce tenant isolation, opt-in, retention,
model provenance, cost budgets, and safe fallback consistently.

## Remaining work and why

Complete and validate prerequisite phases individually, choose operational
providers and data boundaries, then integrate through durable events and
authorized APIs. This remains open because nearly all upstream AI and SaaS
capabilities are still designs, and a unified system cannot be production
validated without actual deployment and evaluation evidence.

## Acceptance criteria

- Each capability has an owner, stable API/schema, model/prompt version,
  evaluation suite, and failure policy.
- Cross-capability data sharing is purpose-limited, consented, and
  tenant-authorized.
- AI failures never block core SMTP acceptance or create unauthorized actions.
- Unified monitoring covers privacy-safe quality, availability, latency, and
  cost.
- Production readiness is backed by security/privacy review, scale tests,
  incident drills, user controls, and explicit release approval.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [Phase 131 AI Platform Foundation](phase-131-ai-platform-foundation.md)
- [Phase 151 AI Search and Semantic Retrieval](phase-151-ai-search-and-semantic-retrieval.md)
