# Phase 140 — Conversation Intelligence

Status: **Threading foundations exist; AI summaries, decisions, and action extraction are not implemented**

## Repository baseline

The platform has Message-ID/References threading foundations and mailbox
search projections. There is no model summarization, action-item extraction,
thread-context service, or approved model data-processing boundary.

## Recommended design

Offer user-invoked or explicitly enabled summaries, decisions, and action
items. Retrieve only messages the requesting principal can access, preserve
thread boundaries, minimize quoted history, and identify source messages for
every extracted claim. Treat content as untrusted prompt input; isolate model
instructions, disable tools, and validate outputs. Summaries are advisory and
must not create tasks, send replies, or change message state without user
confirmation.

## Remaining work and why

Complete threading correctness, consent/provider/retention decisions,
summarization quality and hallucination evaluations, and provenance UI. This
remains open because no model integration, privacy boundary, representative
evaluation set, or user-facing review flow exists.

## Acceptance criteria

- Authorization is checked for every thread message before context assembly.
- Summaries link claims/actions to source messages and mark uncertainty.
- Prompt injection in message text cannot access tools, secrets, or other
  tenant/thread data.
- No generated action is executed without explicit user authorization.
- Quality, omission, hallucination, privacy, and latency are evaluated on a
  consented representative dataset.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [Phase 51 Conversation Threading](phase-51-conversation-threading.md)
