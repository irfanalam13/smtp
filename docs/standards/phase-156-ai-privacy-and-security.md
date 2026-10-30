# Phase 156 — AI Privacy and Security

Status: **Privacy/security principles documented; inference enforcement and audit integration pending**

## Repository baseline

The feature and suggestion contracts avoid exposing raw message fields in
their outputs, but there is no inference service, provider privacy contract,
redaction pipeline, AI audit sink, or model isolation. No cryptographic
technique currently provides a blanket guarantee of privacy-preserving
inference.

## Recommended design

Authorize data before retrieval and again before inference. Minimize context,
redact only with a tested policy, and preserve a safe route to the original
when authorized. Threat-model prompt injection, data exfiltration, model
memorization, cross-tenant caches, provider retention/training, and malicious
model output. Use encryption in transit/at rest, least-privilege runtime
identity, isolated execution, egress controls, and a no-tool default.
Log decision metadata, not content, and integrate with durable audit.

## Remaining work and why

Define data classes, provider/deployment terms, residency, retention, consent,
redaction quality, audit, and incident response. This remains open because no
inference or provider boundary is deployed and privacy rules depend on actual
customer/region context.

## Acceptance criteria

- Authorization is evaluated before context assembly; cross-tenant access
  tests cover retrieval, caches, logs, and evaluation data.
- Provider data use, retention, training, deletion, and residency are
  contractually reviewed.
- Secrets and unnecessary personal data are not included in prompts or logs.
- Output is treated as untrusted and cannot call tools or modify policy.
- Encryption and access-control claims are supported by deployment evidence,
  not assumed from application code.

## Research references

- [OWASP Top 10 for LLM Applications](https://genai.owasp.org/llm-top-10/)
- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 90 Audit and Compliance](phase-90-audit-compliance.md)
