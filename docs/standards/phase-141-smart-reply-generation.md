# Phase 141 — Smart Reply Generation

Status: **Bounded review-only suggestion contract added; model generation and UI are not implemented**

## Repository baseline and code slice

The shared Rust crate now provides `DraftSuggestion` for bounded generated
text, with a suggestion kind, model-version provenance, and optional source
message IDs. It rejects empty/control-character output, oversized text,
duplicate/nil/excessive source IDs, and invalid model-version metadata. It
does not call a model, authorize mailbox access, render HTML, or send mail.

## Recommended design

Generate a small number of short, optional reply candidates from only the
authorized thread context. Respect the user's chosen language, tone and
templates; preserve quoted-message boundaries and mark generated content as a
draft. The model must have no SMTP, contact, calendar, or mailbox mutation
tools. The user must review and explicitly send through the normal submission
path.

## Remaining work and why

Select provider/deployment and data-use terms, define opt-in, prompt and
evaluation policy, build an authorized thread-context service and review UI,
and measure language/tone quality and harmful or fabricated replies. This
remains open because no model, consent policy, integration, or evaluation
corpus exists.

## Acceptance criteria

- Context is authorized per message and excludes unrelated mailbox data.
- Generated text is visibly a suggestion, editable, and never sent
  automatically.
- Prompt injection cannot invoke tools, expose secrets, or bypass submission
  authorization.
- Evaluation covers language, tone, factuality, hallucination and unsafe
  content, with user opt-out and provider failure fallback.
- Suggestion metadata retains source IDs and model version without copying
  message content into logs.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [Phase 131 AI Platform Foundation](phase-131-ai-platform-foundation.md)
