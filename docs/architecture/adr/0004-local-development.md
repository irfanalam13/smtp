# ADR 0004: Local development

- Status: Accepted
- Date: 2026-10-28

## Context

Developers need a consistent database without requiring application services to
run in containers, plus a repeatable first-run and normal-run workflow.

## Decision

Use Docker Compose for PostgreSQL and run Rust, Go, and Next.js natively.
`bootstrap-dev.sh` performs first-run dependency, migration, and build setup;
`run-dev.sh` starts the normal local process set without reinstalling or
rerunning migrations. `deploy-preflight.sh` validates production configuration
and release builds but does not deploy.

## Alternatives considered

Containerizing every process would improve environment parity but slow the edit/test loop.

## Consequences

Docker is required for the database; native toolchains must be installed and
match the documented versions. PostgreSQL remains running after the normal
runner exits so repeated starts are fast. These scripts are not a production
supervisor.
