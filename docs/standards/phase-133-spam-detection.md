# Phase 133 — Spam Detection System

Status: **No spam classifier or production spam policy implemented**

## Repository baseline

The platform has bounded MIME parsing, email authentication foundations, and
rule-engine primitives, but no calibrated spam model, reputation service,
feedback loop, or production quarantine pipeline. Phase 56 spam folder
management is not equivalent to spam detection.

## Recommended design

Combine policy rules, authentication/alignment results, reputation, rate and
behavior signals, and evaluated statistical models. Treat outputs as
confidence-scored signals with reason codes, model/version provenance, and
policy-specific actions. Use tenant-specific overrides only with safe limits.
Provide user review, recovery, false-positive reporting, and abuse safeguards.
Do not train on customer mail without clear authorization and retention
controls.

Spam scores must not alone permanently delete mail or bypass malware and
phishing controls. Keep protected or high-impact decisions auditable and
reversible.

## Remaining work and why

Create a consented, representative labeled dataset; define abuse policy and
evaluation; choose model/feature storage; and integrate an isolated scoring
service with reversible quarantine. This remains open because no model,
dataset, labels, provider, or measured false-positive tolerance exists.

## Acceptance criteria

- Evaluation reports false-positive and false-negative rates by relevant
  language, tenant profile, and message type, with privacy review.
- Thresholds and actions are versioned, explainable, reversible, and
  human-reviewable for high-impact cases.
- Model/provider outages do not block mail acceptance; fallback policy is
  explicit and bounded.
- Feedback is authenticated, abuse-resistant, and excluded from training
  unless separately authorized.
- No irreversible deletion is driven by an unreviewed model score.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [RFC 7489 — DMARC](https://www.rfc-editor.org/rfc/rfc7489)
- [Phase 56 Spam Folder Management](phase-56-spam-folder-management.md)
