# Phase 138 — Priority and Importance Prediction

Status: **No behavioral model or importance prediction implemented**

## Repository baseline

Messages support metadata and priority-related foundations, but no model
learns user interactions, sender importance, urgency, or deadlines. No
behavioral-data consent or model evaluation policy exists.

## Recommended design

Make prioritization opt-in, per-user, explainable, and reversible. Minimize
behavioral history, define short retention windows, and exclude protected or
irrelevant traits. Distinguish machine predictions from explicit user flags
and sender-provided priority headers. Use calibrated confidence and do not
automatically suppress, delete, or send messages based on predicted urgency.

## Remaining work and why

Define interaction data eligible for use, consent and deletion, prediction
quality measures, fairness review, and controls before collecting behavioral
signals. This remains open because personalized inference over private
communications needs explicit product and privacy approval.

## Acceptance criteria

- Users can opt in/out and clear learned signals; predictions never alter
  canonical message content.
- Evaluation measures ranking quality and harm from missed urgent messages.
- User-declared priority is authoritative and not silently overwritten.
- Features and explanations avoid sensitive-trait inference.
- Low confidence or service outage does not hide or delay mail.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 45 Message Metadata](phase-45-message-metadata.md)
- [Phase 131 AI Platform Foundation](phase-131-ai-platform-foundation.md)
