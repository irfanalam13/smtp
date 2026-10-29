# Contributing

Thank you for contributing to SMTP Platform. The repository is an open-source
development and validation foundation; it is not yet a production-ready hosted
email service.

## Prerequisites

- Git
- Rust toolchain declared by [`rust-toolchain.toml`](rust-toolchain.toml)
- Go version declared by [`smtp-api/go.mod`](smtp-api/go.mod)
- Node.js compatible with the dashboard and pnpm 12.15.0
- Docker Engine with Docker Compose v2
- GNU Make

Run `make doctor` from the repository root to inspect the local environment.

## Setup and checks

```bash
cp .env.example .env
make setup
make check
```

`make setup` is safe to repeat and does not overwrite an existing `.env`.
`make check` runs formatting, linting, and the Rust/Go test suites. Use
`make build` for the complete Rust, Go, and dashboard build.

Pull requests run the same checks through GitHub Actions, plus dashboard,
PostgreSQL queue integration, dependency, secret, and workflow-security
validation. See the [CI/CD guide](docs/devops/) for workflow details and
failure diagnosis. Dependabot changes require the ordinary review and checks;
they are not auto-merged.

Focused commands include `make test-api`, `make test-delivery`, and
`make test-queue`. The queue integration test requires an isolated disposable
PostgreSQL database through `TEST_DATABASE_URL`; never point it at a normal
development database.

## Change expectations

- Preserve the architecture and standards decisions in [`docs/architecture/`](docs/architecture/)
  and [`docs/standards/`](docs/standards/).
- Keep changes focused and avoid adding speculative services or dependencies.
- Add or update tests for behavior changes.
- Keep message bodies, credentials, tokens, and private keys out of logs,
  fixtures, and commits.
- Update directly related documentation and report skipped validation.
- Do not enable external SMTP delivery or development authentication at a
  public boundary.

Use the existing language tooling rather than introducing a competing formatter,
linter, task runner, or package manager.

## Pull requests

Describe the user-visible or operational effect, validation commands and
results, configuration changes, migration implications, and known limitations.
Reviewers should be able to reproduce the checks from a clean checkout.

## Development environment failures

Report the exact command, tool versions from `make doctor`, relevant sanitized
logs, and whether the failure reproduces after `make infra-down` followed by
`make infra-up`. Do not attach `.env`, credentials, raw message content, or
database dumps.
