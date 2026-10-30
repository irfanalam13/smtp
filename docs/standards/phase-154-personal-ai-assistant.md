# Phase 154 — Personal AI Assistant

Status: **No mailbox assistant, tool gateway, or secure command execution implemented**

## Repository baseline

Bounded suggestions and semantic ranking primitives exist, but there is no
assistant orchestration, mailbox retrieval adapter, model, tool framework, or
permission-aware command executor.

## Recommended design

Treat the assistant as an untrusted natural-language interface. Resolve each
request into a typed intent, display planned operations, and require explicit
confirmation for side effects. Search only resources authorized for the
current principal; re-check access before returning content. Use allowlisted,
least-privilege tools with strict schemas, argument validation, audit, and
timeouts. Never expose secrets or allow email text to modify system policy.

## Remaining work and why

Complete tenant identity, semantic search/indexing, model governance, and a
reviewable tool/approval boundary. This remains open because none of the
production retrieval, inference, or authorization-aware action services are
available.

## Acceptance criteria

- Every answer is grounded in authorized sources and cites source messages.
- Tool calls are allowlisted, typed, scoped, rate-limited, and re-authorized.
- Sending, deleting, forwarding, calendar changes, and workflow execution
  require user confirmation.
- Prompt injection cannot escape the tool boundary or retrieve other tenants.
- Assistant history follows user controls, retention, and deletion policy.

## Research references

- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [Phase 151 Semantic Retrieval](phase-151-ai-search-and-semantic-retrieval.md)
- [Phase 156 AI Privacy and Security](phase-156-ai-privacy-and-security.md)
