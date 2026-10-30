# Phase 143 — AI Email Summarization

Status: **Review-only summary output type available; summarization model and product flow are not implemented**

## Repository baseline

The shared `DraftSuggestion` type can carry a bounded summary tied to source
message IDs. The platform has MIME parsing and conversation-threading
foundations, but it does not summarize messages, attachments, newsletters, or
threads.

## Recommended design

Summarize only content the requesting user is authorized to read. Preserve
source references for each important claim/action, distinguish quoted content
from the latest message, and clearly label uncertainty. Parse attachments
under independent resource limits and use scanners before any text extraction.
Do not treat model output as evidence, a legal record, or an instruction to
perform an action.

## Remaining work and why

Choose supported inputs, provider and data policy; build thread/attachment
context selection; and evaluate hallucinations, omissions, source attribution,
languages, and long-thread truncation. This remains open because no model,
evaluation corpus, authorization-aware retrieval, or provenance UI exists.

## Acceptance criteria

- Every summary is bounded and linked to the exact source message(s).
- Unsupported, encrypted, or unscanned attachment content is not silently
  presented as summarized.
- Evaluations measure factual consistency, omissions, uncertainty, and
  prompt-injection resilience.
- No summary is used to delete, send, classify, or change retention state.
- User can request regeneration, report errors, and opt out; raw content is not
  logged.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 47 MIME Parsing](phase-47-mime-parsing.md)
- [Phase 51 Conversation Threading](phase-51-conversation-threading.md)
