# Phase 131 — AI Platform Foundation

Status: **Offline deterministic feature-extraction slice added; model gateway and inference platform not implemented**

## Repository baseline and code slice

The `smtp-ai` package is a placeholder. The bounded Rust MIME parser produces
`ParsedMessage` projections, and `shared-types::ai_features` now derives a
small `EmailFeatures` value containing aggregate counts only. The extractor
does not call a model, store message content, fetch URLs, or execute
attachments. It is an input-feature foundation, not a classifier or
production AI service.

## Recommended design

Keep model providers behind an internal gateway with tenant/resource
authorization, purpose and consent checks, data minimization, residency and
retention controls, provider training restrictions, time/cost limits, output
validation, and explicit fallback. Separate CPU/GPU inference from SMTP
acceptance and mailbox transactions. Use durable asynchronous jobs for
non-interactive analysis; never block durable mail acceptance on model
availability. Maintain model/version provenance and evaluation gates.

Treat email and attachment data as untrusted, including prompt injection.
Models have no tools or external side effects by default. AI output must not
send, delete, forward, or change security policy without an independently
authorized human action.

## Remaining work and why

Approve use cases, data eligibility, providers, deployment, privacy,
residency, evaluation, and cost ownership. Implement a gateway only after
these boundaries are agreed. This remains open because mail content is
sensitive and no provider, consent model, production service, or inference
operations are configured.

## Acceptance criteria

- Tenant/resource authorization is enforced before feature access or inference.
- Data sent to a provider is minimized, disclosed, retained, and deleted under
  approved policy; model training use is explicitly controlled.
- Provider failure, timeout, malformed output, or budget exhaustion degrades
  safely without blocking mail delivery or mailbox reads.
- Model and prompt versions, evaluation results, latency, cost, and safety
  outcomes are traceable without logging raw message bodies.
- Consequential operations always require ordinary authorization and human
  confirmation.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [Phase 95 AI Platform](phase-95-ai-platform.md)
- [MIME parsing boundary](phase-47-mime-parsing.md)
