# Phase 95 — AI Platform

Status: **No AI inference, model-provider integration, or AI-driven mail feature implemented**

## Repository baseline

The `smtp-ai` package is a placeholder. No model provider, inference service,
feature pipeline, prompt store, data-retention policy, evaluation set, or
AI-generated mail UI is present.

## Recommended AI safety boundary

Treat email, attachments, templates, and retrieved tenant data as untrusted
input, including indirect prompt injection. Models must not receive secrets,
other-tenant data, or message content unless explicitly authorized and
necessary. Retrieval must apply tenant/resource authorization before context
construction. Model output is untrusted: validate structured outputs, escape
rendered text, and require human approval before sending, deleting, changing
security policy, or performing external side effects.

Start with opt-in, narrow assistive features such as draft summaries or
categorization, with explicit data-use disclosure, retention and deletion
controls, provider training opt-out/contract review, tenant isolation, and
fallback behavior. Spam classification remains a defense-in-depth signal
integrated with established policy/scanning, not an autonomous irreversible
decision. Track latency/cost limits, provenance, confidence, abuse, and
quality/fairness evaluations. Support on-prem/private models only if
deployment requirements and operational ownership justify them.

## Remaining work and why

Define feature use cases, data eligibility/consent, model/vendor, residency,
retention, prompt/output constraints, human oversight, evaluation datasets,
abuse monitoring, fallback, cost budgets, and incident response. Threat-model
prompt injection and exfiltration. Build an isolated inference adapter and
feature-level tests before customer exposure.

This remains open because there is no AI product policy, provider, or
approved data-processing boundary. Mailboxes contain highly sensitive
communications; sending them to a model or allowing model actions without
explicit controls would create privacy, security, and reliability risks.

## Acceptance criteria

- Tenant/resource authorization is applied before retrieval and model calls;
  no cross-tenant context can enter a prompt.
- Sensitive data handling, provider training, retention, residency, and
  opt-in/opt-out are documented and testable.
- Prompt injection cannot authorize tools, expose secrets, or directly invoke
  consequential actions.
- Outputs are validated and human-approved for consequential mail operations.
- Evaluations measure quality, false positives, latency, cost, privacy, and
  safe fallback on a representative, consented dataset.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [AI package placeholder](../../smtp-ai/README.md)
