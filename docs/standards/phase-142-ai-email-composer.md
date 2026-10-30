# Phase 142 — AI Email Composer

Status: **Review-only output validation foundation; composer features and model service are not implemented**

## Repository baseline

`DraftSuggestion` can represent bounded compose-assistance text and model
provenance, but there is no UI, inference adapter, grammar checker, translation,
or formatting service. The type does not sanitize HTML or prove factual
correctness.

## Recommended design

Offer user-triggered rewrite operations (grammar, tone, expansion,
summarization, translation) with a preview and undo. Preserve original draft
content, delimit quoted material, and never insert content into a sendable
message without visible user confirmation. Escape rendered output and keep
plain-text and HTML modes separate. Enforce data-use disclosure, tenant
configuration, cost/length limits, and keyboard-accessible review.

## Remaining work and why

Define content retention/provider policies, supported languages and quality
targets, product controls, and safe rendering. Implement one opt-in operation
at a time with offline evaluation and fallback. This remains open because
there is no model provider, UI, consent model, or quality evidence.

## Acceptance criteria

- User selects the exact draft region and operation; no hidden whole-mailbox
  context is included.
- Original text is recoverable and generated changes are previewed and
  reversible.
- Output is bounded, encoded for the target content type, and checked for
  malformed or unsafe markup.
- The application never sends a draft or executes an action based on a model
  response.
- Provider outage, timeouts, and unsupported languages preserve the user's
  original draft.

## Research references

- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [Phase 59 Draft Management](phase-59-draft-management.md)
- [Phase 131 AI Platform Foundation](phase-131-ai-platform-foundation.md)
