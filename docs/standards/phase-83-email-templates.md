# Phase 83 — Email Templates

Status: **No reusable template service implemented**

## Repository baseline

The repository supports raw RFC message handling, bounded MIME parsing, and
the SMTP/API infrastructure. There is no tenant-scoped template model,
renderer, MJML compiler, localization catalog, preview service, version
history, or reusable component system.

## Recommended template contract

Keep templates separate from raw SMTP protocol handling. Define immutable
versions with tenant ownership, draft/published state, typed variable
schemas, explicit locale/fallback rules, sender/domain authorization,
preview/test rendering, and audit history. Rendering should be deterministic
and bounded by input size, nesting, execution time, and output size.

Use a maintained MJML compiler only if responsive email authoring is required.
Treat Handlebars-like syntax as a constrained presentation language: no
arbitrary code, filesystem/network access, dynamic helper registration, or
unbounded recursion. Escape variables by default; HTML output remains
untrusted and needs safe preview isolation. Plain-text output should be
available, and rendered messages must pass through the same submission,
quota, abuse, and delivery policy as non-template messages.

## Remaining work and why

Define template ownership, variable/type system, editor UX, localization and
fallback policy, branding inheritance, version/publish/rollback behavior,
preview isolation, compiler choice/license, and send integration. Then
implement tenant-scoped persistence, rendering limits, HTML sanitization
policy, previews, and deterministic fixture tests.

This remains open because there is no product contract for template editing,
localization, or publishing, and arbitrary template execution could become a
server-side code execution or HTML injection surface. A template renderer
must not bypass existing sender and delivery controls.

## Acceptance criteria

- A template is tenant-scoped, versioned, audited, and immutable when
  published; rollback is explicit.
- Unknown/missing variables, oversized input/output, deep expressions, and
  malformed templates fail clearly without partial delivery.
- Variables are escaped in HTML context; previews cannot access internal
  network or filesystem resources.
- Rendering is deterministic across retries and includes a plain-text
  alternative where required.
- Template send paths enforce authenticated tenant, sender, quotas, abuse,
  and idempotency policy.

## Research references

- [MJML documentation](https://documentation.mjml.io/)
- [OWASP Cross Site Scripting Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html)
- [Phase 47 MIME parsing](phase-47-mime-parsing.md)
