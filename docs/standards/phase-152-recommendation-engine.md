# Phase 152 — Recommendation Engine

Status: **No recommendation models or user-behavior pipeline implemented**

## Repository baseline

Mailbox labels, templates, contacts, and rules provide product primitives, but
there is no recommender, behavioral event store, consent policy, or ranking
evaluation. Communication frequency is not proof of trust, relevance, or
permission.

## Recommended design

Start with user-requested, explainable recommendations and explicit controls.
Keep suggestions separate from user state; require review before changing
recipients, labels, folders, templates, or workflows. Minimize and expire
behavioral signals, support correction/reset/opt-out, and prevent cross-tenant
training or retrieval. Security recommendations must not infer sender safety
from popularity or prior interaction alone.

## Remaining work and why

Define use cases, eligible events, data retention, consent, ranking objectives,
and harm/error evaluation. This remains open because no user behavior data
policy, recommendation model, or feedback process exists.

## Acceptance criteria

- Every suggestion has provenance, confidence, and a user explanation/control.
- No suggestion silently sends mail, changes access, or triggers automation.
- Training and inference are tenant-isolated and follow opt-in/retention policy.
- Evaluation measures relevance, bias, privacy leakage, and harmful outcomes.
- Users can dismiss, correct, reset, and disable recommendations.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 150 Contact Relationship Intelligence](phase-150-contact-relationship-intelligence.md)
- [Phase 159 Enterprise AI Governance](phase-159-enterprise-ai-governance.md)
