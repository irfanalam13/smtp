# ADR-0501: CI/CD boundary and release trust

- Status: Accepted
- Date: 2026-10-28

## Decision

Use GitHub Actions for repository validation because the repository already
contains a GitHub workflow and no alternative CI provider is configured.
Pull-request validation is unprivileged and separate from release publication.
Release publication is allowed only from an explicitly validated
`vMAJOR.MINOR.PATCH` tag or a manual run targeting an existing version tag.

Phase 05 validates the existing Rust, Go, dashboard, and PostgreSQL workflows.
It does not deploy production services, provision cloud infrastructure, publish
containers, or access production secrets.

## Consequences

The pipeline is reproducible and has a clear trust boundary, but remote branch
protection and action SHA pinning still require repository administration.
Release publication requires GitHub repository settings that permit the
workflow token to create releases.
