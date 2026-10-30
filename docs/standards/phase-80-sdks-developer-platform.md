# Phase 80 — SDKs and Developer Platform

Status: **SDK and CLI package boundaries reserved; generated clients and developer lifecycle pending**

## Repository baseline

The repository has reserved `smtp-sdk` and `smtp-cli` packages. The current
Go API has an OpenAPI document, but generated clients, official language
packages, a versioned CLI, Postman collection, and developer onboarding
workflow are not complete. The API contract is not yet stable across the full
SaaS resource surface.

## Recommended SDK strategy

Generate language clients from the reviewed, versioned OpenAPI contract
where practical; keep hand-written code limited to a small, tested transport
and ergonomic helpers. Do not maintain seven independent implementations of
the same endpoint semantics. Publish a support matrix and release compatible
packages for JavaScript, TypeScript, Python, Go, Java, PHP, and C# in staged
order. Generated artifacts must be reproducible and reviewed for unsafe
defaults; generated code is not automatically secure or ergonomic.

Provide consistent authentication configuration, bounded timeouts, retries
only for safe/idempotent operations, cancellation, pagination, typed errors,
request IDs, and no logging of credentials or message bodies. API keys must
be supplied through secure runtime configuration, never embedded in examples
or committed files. The CLI should reuse API contracts and SDK transport, and
must make destructive or billing actions explicit.

## Remaining work and why

Stabilize API contracts and publish policy; choose generator/tooling and package
release channels; define semantic-versioning and deprecation guarantees; add
generated-client drift checks, language-specific tests, examples, Postman
artifacts, CLI command policy, and onboarding documentation. Start with
languages used by real consumers, then add the rest against the same
conformance suite.

This remains open because the current API is partial and still evolving.
Publishing clients against unstable routes creates costly compatibility
breakage; generated clients without contract tests can be syntactically valid
but behaviorally unsafe or inconsistent.

## Acceptance criteria

- Each officially supported language passes shared contract tests for
  authentication, errors, pagination, timeouts, and idempotent retries.
- SDK generation is deterministic from the checked-in OpenAPI version and
  drift is detected in CI.
- Package releases follow semantic versioning and document supported API
  versions and language runtimes.
- Samples use placeholders and secure key-loading patterns; no real secrets
  appear in repository history or generated docs.
- CLI output and diagnostics avoid leaking credentials or message contents.

## Research references

- [OpenAPI Specification 3.1.1](https://spec.openapis.org/oas/v3.1.1.html)
- [Reserved SDK package](../../smtp-sdk/README.md)
- [Reserved CLI package](../../smtp-cli/README.md)
- [Current API guide](../api/README.md)
