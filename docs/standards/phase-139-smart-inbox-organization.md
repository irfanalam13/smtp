# Phase 139 — Smart Inbox Organization

Status: **No AI-driven folder, tab, queue, or workflow organization implemented**

## Repository baseline

The platform has mailbox folders, labels, rules, and organization foundations,
but no AI orchestration or personalized smart inbox. Phases 131–138 are
design/feature foundations only; the current extractor does not classify
messages.

## Recommended design

Use non-destructive virtual views over canonical mailbox state. Make automated
organization opt-in, explainable, previewable, reversible, and subordinate to
user rules and legal retention. Keep tenant/user data isolated when ranking or
clustering. Do not move messages across security boundaries or modify mailbox
permissions based on AI output.

## Remaining work and why

Complete classification/priority evaluations, user-consent and correction
flows, conflict handling with explicit rules, and mailbox event integration.
This remains open because upstream model quality and privacy controls are not
implemented.

## Acceptance criteria

- AI organization is a view or reversible operation and never destroys or
  silently relocates canonical data.
- User rules and explicit folder choices have documented precedence.
- Every suggestion has provenance and a user control to dismiss/correct it.
- Cross-tenant retrieval and ranking are prevented and tested.
- Outages and low confidence preserve the ordinary inbox behavior.

## Research references

- [Phase 43 Folder Management](phase-43-folder-management.md)
- [Phase 54 Labels and Categories](phase-54-labels-and-categories.md)
- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
