# Phase 148 — Task and Action Item Extraction

Status: **No action-item extraction or task creation integration implemented**

## Repository baseline

There is no action-item model or task integration. The Phase 140 design
requires source-linked summaries, but no model-backed thread context or
provenance UI exists.

## Recommended design

Extract candidate action, assignee, due date, and source span with confidence
and uncertainty; distinguish explicit commitments from suggestions. Treat
inferred tasks as drafts in a user review queue. Never send reminders, assign
work, approve requests, or create external tasks without explicit user
confirmation and ordinary authorization.

## Remaining work and why

Define the task schema, ownership resolution, consent, retention, and target
integrations. Evaluate extraction and false-assignment rates across thread
formats and languages. This remains open because no task product, model,
provider, or review workflow is selected.

## Acceptance criteria

- Every candidate action links to source message and text span, with confidence.
- Assignee and deadline can remain unknown; ambiguity is not fabricated.
- Users review, edit, dismiss, or confirm before task creation or notification.
- Integration tokens are never exposed to model prompts.
- Thread access and tenant authorization are checked before extraction and
  again before any confirmed external action.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 140 Conversation Intelligence](phase-140-conversation-intelligence.md)
- [Phase 84 Automation Engine](phase-84-automation-engine.md)
