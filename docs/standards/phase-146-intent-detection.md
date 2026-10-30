# Phase 146 — Intent Detection

Status: **No intent taxonomy or classifier implemented**

## Repository baseline

There are rule-engine and mailbox-label foundations but no intent model,
taxonomy, training data, or automation integration. An inferred intent is
uncertain and must not be treated as a sender's verified commitment or legal
notice determination.

## Recommended design

Define a small, versioned taxonomy with multi-label and unknown outcomes,
confidence, provenance, and source references. Distinguish machine-inferred
intent from explicit user labels and deterministic business rules. Keep
classification opt-in and reversible; do not trigger customer support, legal,
payment, recruitment, or workflow actions without review and policy checks.

## Remaining work and why

Agree taxonomy, intended use, privacy and fairness review, labeled data, and
evaluation thresholds. This remains open because the repository has no
business policy, representative labels, model, or measured harm tolerance.

## Acceptance criteria

- Taxonomy is versioned, bounded, multi-label where needed, and supports
  unknown/ambiguous outcomes.
- Evaluation reports per-class precision/recall and errors across languages
  and tenant contexts.
- Predictions remain separate from user labels and are editable/reversible.
- No inferred intent starts external or irreversible workflow automatically.
- Model/provider failures leave existing mail behavior unchanged.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 53 Rule Engine](phase-53-rule-engine.md)
- [Phase 137 Email Classification](phase-137-email-classification.md)
