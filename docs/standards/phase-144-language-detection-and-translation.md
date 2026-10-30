# Phase 144 — Language Detection and Translation

Status: **No language detector or translation service implemented**

## Repository baseline

MIME parsing decodes message text but does not detect language or translate
content. `DraftSuggestion` can represent a review-only translation output;
it does not validate BCP 47 language tags, preserve HTML structure, or verify
translation quality.

## Recommended design

Detect language with confidence and an explicit "unknown/mixed" result.
Prefer user-selected language when available and do not infer locale from
identity or region without user consent. Translate a derived view or draft;
keep canonical message bytes and attachments unchanged. Preserve markup via a
safe structured representation, not string replacement. Redact or avoid
transmitting protected content according to policy and provider terms.

## Remaining work and why

Select language and translation providers, supported language pairs, data
residency, quality thresholds, and user controls. Implement with a versioned
BCP 47 tag contract and fixtures for short, mixed-language, quoted, and
malformed text. This remains open because there is no detector, provider,
evaluation set, or content-processing boundary.

## Acceptance criteria

- Detection returns confidence and unknown/mixed outcomes for ambiguous input.
- Language identifiers follow [BCP 47](https://www.rfc-editor.org/rfc/rfc5646).
- Translation preserves the original and does not alter attachments or
  canonical MIME storage.
- HTML output is structurally sanitized and never trusts model markup.
- Supported-language quality, privacy, latency, and provider outage behavior
  are measured and documented.

## Research references

- [RFC 5646 — Language Tags](https://www.rfc-editor.org/rfc/rfc5646)
- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
- [Phase 47 MIME Parsing](phase-47-mime-parsing.md)
