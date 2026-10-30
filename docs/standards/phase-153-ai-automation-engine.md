# Phase 153 — AI Automation Engine

Status: **No AI-triggered workflow engine implemented**

## Repository baseline

Rules and workflow architecture documents exist, but there is no AI classifier
pipeline or production automation scheduler. AI outputs are not verified facts
and cannot be treated as authorization.

## Recommended design

Separate model inference from policy evaluation and side effects. Feed
validated, versioned signals into a deterministic rule engine with tenant
scoping, explicit conditions, idempotency, dry-run, audit, rate limits, and
rollback. Require human approval for external communication, financial/legal
actions, access changes, deletion, or low-confidence outcomes. Prompt
instructions must never directly invoke tools or bypass authorization.

## Remaining work and why

Complete tenant-safe eventing, AI evaluation, policy/approval workflow,
idempotent execution, and operator controls. This remains open because there
is no event platform, model, approval service, or production audit trail.

## Acceptance criteria

- Model scores are inputs, not executable commands or permission grants.
- Every action is re-authorized at execution and audited with provenance.
- Retries are idempotent and bounded; users can preview, pause, and revoke.
- High-impact actions require approval; model/provider failure cannot trigger
  a success-shaped fallback.
- Tenant isolation, prompt injection, replay, and duplicate-event tests pass.

## Research references

- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [Phase 84 Automation Engine](phase-84-automation-engine.md)
- [Phase 159 Enterprise AI Governance](phase-159-enterprise-ai-governance.md)
