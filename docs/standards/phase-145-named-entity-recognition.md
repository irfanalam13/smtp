# Phase 145 — Named Entity Recognition

Status: **No entity extraction or entity storage implemented**

## Repository baseline

The MIME parser exposes decoded text and addresses, but there is no NER
pipeline, entity store, policy, or review UI. Entities such as names,
locations, phone numbers, invoice IDs, and contract references can be
particularly sensitive; feature extraction must not be confused with consent
to persist them.

## Recommended design

Make entity extraction opt-in and purpose-specific. Return typed entity
categories, confidence, and source offsets/message IDs; minimize raw values
and apply retention/deletion policy. Keep entities tenant/mailbox scoped and
use authorization before indexing, retrieval, or export. Require human review
before using entities for routing, billing, legal, or employment decisions.

## Remaining work and why

Determine lawful purpose, categories, data retention and user controls; select
models and evaluate precision/recall across languages and formats. This
remains open because there is no model, policy, data inventory, or evaluation
set, and extracted entities create a new sensitive-data store.

## Acceptance criteria

- Entity categories and retention are explicit; sensitive categories are
  excluded unless specifically approved.
- Outputs preserve provenance and confidence and are not treated as verified
  facts.
- Tenant/mailbox access control applies to both values and source offsets.
- Evaluation includes false extraction, missed values, language and
  formatting-specific performance.
- No consequential workflow is triggered without user confirmation.

## Research references

- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 132 Email Feature Extraction](phase-132-email-feature-extraction.md)
- [Phase 90 Audit and Compliance](phase-90-audit-compliance.md)
