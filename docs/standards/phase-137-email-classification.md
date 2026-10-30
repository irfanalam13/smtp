# Phase 137 — Email Classification Engine

Status: **No message classifier or user-label prediction implemented**

## Repository baseline

Mailbox labels, folders, and rules provide user-managed organization. No
classifier, training/evaluation pipeline, personalized prediction, or
classification feedback service exists.

## Recommended design

Begin with a small, explainable taxonomy and opt-in predictions. Keep model
labels distinct from user-created labels; never overwrite user organization.
Store confidence, model/version, and provenance. Apply tenant-specific labels
only after authorized mailbox access, and let users correct or disable
predictions. Evaluate language, region, organization, and mailbox-type
performance.

## Remaining work and why

Define taxonomy, user controls, datasets, privacy policy, and evaluation;
then implement an asynchronous classifier and review UI. This remains open
because there is no approved taxonomy, consented training/evaluation corpus,
or product behavior for uncertain predictions.

## Acceptance criteria

- Predictions are opt-in/configurable, reversible, and separate from user data.
- Confidence and explanation are available where decisions affect workflow.
- Evaluation includes representative languages, tenants, and error analysis.
- Low-confidence or unavailable classifications leave mail unchanged.
- User corrections are not used for training without explicit policy and
  authorization.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 54 Labels and Categories](phase-54-labels-and-categories.md)
- [Phase 132 Email Feature Extraction](phase-132-email-feature-extraction.md)
