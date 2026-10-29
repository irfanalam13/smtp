# GitHub Copilot Implementation Prompt: Connect Go SMTP API to PostgreSQL Mailbox Storage

## Role

Act as a senior Go, Rust, PostgreSQL, and distributed-systems engineer working directly inside this existing repository:

`/run/media/irru/B0A0D1E9A0D1B656/Projects/smtp-platform`

Your task is to **inspect, implement, test, and document the integration between the Go API (`smtp-api`) and the existing PostgreSQL-backed mailbox system**. Work with the repository as it exists today. Do not assume that this prompt's descriptions are more accurate than the actual source code.

## Problem statement

The Go API currently has mailbox/message paths that use an in-memory control-plane store, and its development authentication/identity path may not be connected to durable PostgreSQL identities and permissions. The repository also contains a Rust mailbox service/store and PostgreSQL migrations defining some control-plane entities.

The desired architecture is:

```text
Client
  -> Go HTTP API
  -> authenticate request and resolve durable principal
  -> authorize tenant, user, API key scope, and mailbox access
  -> Go PostgreSQL repository
  -> PostgreSQL mailbox/message data
```

The Rust mailbox service may continue using its own Rust repository against the same PostgreSQL database. **Do not make Go import Rust code or call Rust directly unless repository evidence proves that is already an intentional architectural requirement.** Sharing PostgreSQL is the initial integration approach, provided both implementations use compatible schema and transaction/ownership rules.

## Non-negotiable rules

1. **Inspect before editing.** Read the relevant files, tests, migrations, configuration, and architecture decision records before choosing implementation details.
2. **Do not invent schema.** Use actual migration files and existing Rust SQL/query code to establish table names, columns, constraints, enum values, and relationships.
3. **Preserve existing behavior.** Avoid broad rewrites, unrelated refactors, route renaming, or response-shape changes.
4. **Use the existing project conventions.** Follow the repository's package structure, logging, configuration, error handling, dependency injection, migration process, and test style.
5. **Security first.** Never authorize access based only on `user_id`, `tenant_id`, or ownership values supplied by the client.
6. **No fake completion.** Run the relevant formatters and tests. Report exactly what was and was not verified.
7. **Do not print or commit secrets.** Never place database credentials, API keys, JWT secrets, or real user data in source code, test output, or documentation.
8. **Do not reset, drop, or destructively migrate a database.** Never run destructive database operations against an environment whose safety is uncertain.
9. **Keep scope focused.** Implement one vertical slice first: a read-only mailbox/message endpoint. Expand only after that slice is working and tested.
10. **Do not silently bypass failing tests or weaken security checks** to make the implementation pass.

## Phase 1 — Repository reconnaissance

Before editing, inspect at least:

- `smtp-api/cmd/api/main.go`
- `smtp-api/go.mod`
- `smtp-api/internal/controlplane/controlplane.go`
- Existing HTTP handlers and handler tests, especially mailbox/message routes
- All PostgreSQL migrations under `smtp-storage/postgres/`
- `docs/architecture/adr/0002-service-boundaries.md`
- Rust source containing `MailboxStore` (locate it with repository search)
- Rust mailbox/message repository methods and SQL statements
- Existing configuration/environment handling, authentication middleware, principal types, API-key validation, and tenant membership logic
- Existing test setup and database integration-test conventions

Use repository search tools such as `rg` and `find` as appropriate. Do not assume a file path exists just because it is mentioned in earlier discussion.

Write down a short implementation map before coding:

1. Current route and handler for listing mailbox messages.
2. Current in-memory storage and how it is initialized.
3. Existing PostgreSQL tables and foreign-key relationships for users, tenants, memberships, API keys, mailboxes, messages, and permissions.
4. Existing principal/authentication contract and where it can be extended.
5. Rust's authoritative schema/query expectations.
6. Existing tests that must remain passing.

If a critical relationship is genuinely absent or contradictory, explain the issue and implement only the safe, unambiguous portion. Do not fabricate a migration or authorization policy without documenting the decision.

## Phase 2 — PostgreSQL connection and lifecycle

Inspect whether a PostgreSQL driver/pool is already used in `smtp-api`. Reuse it if appropriate; do not add a duplicate driver unnecessarily.

If no suitable pool exists, choose a compatible production-ready driver/pool consistent with the Go version and project dependencies (for example, `pgx/v5/pgxpool`).

Requirements:

- Read the connection string from the project's established configuration system or a documented environment variable such as `DATABASE_URL`.
- Do not hardcode credentials.
- Parse and validate configuration at startup.
- Create a connection pool with configurable or reasonable pool limits.
- Verify connectivity with a bounded timeout.
- Fail startup clearly if the database is required but unavailable; do not silently fall back to in-memory storage in production.
- Close the pool on shutdown and integrate with the existing graceful-shutdown lifecycle.
- Use request contexts and context-aware database calls.
- Keep connection settings consistent with the repository's local-development and deployment configuration.
- Use TLS for remote/production PostgreSQL connections as required by the deployment environment. Do not recommend disabling TLS in production.
- Add or update `.env.example`/configuration documentation with placeholders only, if the repository uses such files.
- Do not run migrations automatically at API startup unless that is already an explicit project convention. Follow the established migration workflow.

## Phase 3 — Go mailbox repository

Implement a dedicated Go repository for the read-only mailbox/message path. Use existing package naming and architecture where possible; do not create redundant layers merely to match this prompt.

Requirements:

- Inject the repository into the existing handler using the project's established dependency-injection style.
- Use parameterized SQL or safe query-builder methods. Never concatenate client input into SQL.
- Use actual table and column names verified from migrations and Rust queries.
- Select only fields needed by the existing API response.
- Preserve the existing route, JSON response shape, pagination conventions, sorting, and error contract unless a change is required for correctness or security.
- Handle no rows, malformed IDs, query failures, and context cancellation consistently with the API's conventions.
- Apply bounded pagination and stable ordering if the endpoint supports listing many messages.
- Avoid N+1 queries where a straightforward join or batched query is appropriate.
- Do not load message bodies or attachments unnecessarily if the endpoint only returns message metadata.
- Use explicit transactions only where they are actually needed.
- Avoid coupling Go code to Rust implementation details; the shared contract is the documented schema and agreed domain semantics.
- Add repository unit tests for logic that can be isolated and PostgreSQL integration tests for actual SQL behavior.

### First vertical slice

Start with the existing route corresponding to:

`GET /v1/mailboxes/{mailbox_id}/messages`

Treat this as an example route, not permission to add a duplicate if the repository uses a different canonical route. Find the real route and modify that route in place.

Initially support a safe, read-only list operation. Do not attempt to implement every mailbox feature in the same change.

## Phase 4 — Durable identity and authentication

Trace the current request authentication path end to end before modifying it.

Requirements:

- Distinguish development/test authentication from production authentication.
- Do not allow an in-memory development identity to authorize production mailbox reads.
- Reuse existing JWT/session/API-key validation rather than implementing a parallel auth mechanism.
- Resolve the authenticated external identity to the durable user record using the repository's actual identity mapping (for example, `external_subject` only if the schema and current identity provider use it).
- Resolve tenant membership and account status from authoritative records.
- Validate API keys using the existing secure key-storage/verification scheme. Do not store or compare plaintext keys if the current design uses hashed keys.
- Enforce expiration, revocation, status, and scopes where supported by the existing schema and auth design.
- Put the verified principal into request context using the project's existing pattern.
- Ensure missing or invalid authentication is rejected before database mailbox data is returned.
- Do not treat successful authentication as proof of mailbox authorization.

If API keys are tenant-scoped but mailbox permissions are user-scoped, do not assume tenant membership grants access to every mailbox. Inspect the schema and ADR, then implement an explicit policy consistent with the system's intended service-account model. If the schema cannot safely represent the required relationship, document the gap and propose the smallest backward-compatible migration instead of silently granting broad access.

## Phase 5 — Mailbox authorization and tenant isolation

Every mailbox read must enforce authorization using the verified principal and authoritative database relationships.

Requirements:

- Never trust caller-supplied user/tenant ownership IDs.
- Confirm the mailbox belongs to the correct tenant and the principal has the required permission.
- Use the actual mailbox ownership/membership/permission schema.
- Check the relevant read/list scope or permission.
- Prevent cross-tenant reads even when a caller guesses another mailbox ID.
- Prefer enforcing authorization in the query or through a clearly defined authorization query followed by a safe data query.
- Ensure the data query itself is tenant/mailbox scoped; do not fetch broad data and filter it in application memory.
- Use parameterized SQL throughout.
- Return the API's established safe error response for inaccessible resources; avoid leaking mailbox existence where the current security policy requires indistinguishable responses.
- If PostgreSQL row-level security already exists, preserve and correctly initialize it. Do not claim RLS protection unless policies are present and correctly applied to the actual application role.
- If introducing RLS is outside this first slice, explicitly note it as a possible defense-in-depth follow-up rather than a substitute for application authorization.

## Phase 6 — API-key and permission policy

Inspect the actual control-plane migration and current API-key design.

The implementation must explicitly answer in code comments or documentation:

1. Is an API key owned by a user, tenant, or service account?
2. How are the key's scopes checked?
3. How does the key's principal obtain mailbox read permission?
4. Can any key access all mailboxes in a tenant? If yes, what explicit privilege and policy permits this?
5. How are revoked, expired, or disabled keys handled?

Do not silently turn a tenant-level key into a tenant-wide mailbox access grant. If the current schema needs a creating-user/service-account relation, use a backward-compatible migration only after validating the intended policy and the project's migration naming/order conventions. Include rollback/down migration where the repository expects one.

## Phase 7 — Tests

Add or update tests following existing project conventions.

At minimum verify:

### Database and repository
- Startup fails clearly when required database configuration is missing or invalid.
- Connectivity failures are handled without leaving leaked pools/goroutines.
- Repository query succeeds with valid schema and fixture data.
- Empty mailbox results are handled correctly.
- Invalid IDs and database errors are mapped to the established API error contract.
- Pagination/sorting is stable and bounded where supported.

### Authentication
- Missing credentials are rejected.
- Invalid, expired, revoked, disabled, or insufficient-scope credentials are rejected where applicable.
- A valid credential resolves to the correct durable principal.
- Development-only authentication cannot authorize production requests.

### Authorization and isolation
- A permitted user can read their mailbox messages.
- A user without mailbox permission cannot read them.
- A principal from tenant A cannot read tenant B's mailbox.
- A guessed mailbox ID does not bypass authorization.
- A tenant-scoped API key cannot gain user-level mailbox access unless the documented policy explicitly grants it.
- Client-supplied owner or tenant IDs cannot override the authenticated principal.

### Regression
- Existing Go tests continue to pass.
- Existing route and response behavior is preserved.
- Rust service code/schema assumptions remain compatible.
- Tests do not require production credentials or a production database.

Use an isolated test database, disposable PostgreSQL instance, or existing integration-test mechanism. Never point destructive tests at a shared or production database. If PostgreSQL integration tests cannot run in the current environment, still add them and clearly report that they were not executed.

## Phase 8 — Logging, errors, and operations

- Use the project's existing structured logger.
- Log useful operational details without logging credentials, bearer tokens, raw API keys, message contents, or sensitive attachment data.
- Do not expose SQL statements containing secrets or internal stack traces in HTTP responses.
- Distinguish authentication failures, authorization denials, and internal database errors in logs.
- Follow the existing health/readiness endpoint design. If the API has readiness checks, consider database availability there without leaking connection details.
- Document required environment configuration and how to apply existing migrations.
- Do not claim production readiness based only on a successful local connection.

## Phase 9 — Verification commands

Use the repository's actual Go module/workspace layout. From `smtp-api`, run applicable checks such as:

```bash
gofmt -w <changed-go-files>
go test ./...
go vet ./...
```

Run repository-defined lint/build commands if present. Run Rust tests only if the changes affect Rust code, shared schema, or compatibility and the toolchain is available.

Do not run formatters over unrelated files. Review the final diff for accidental changes, secrets, generated files, or unrelated formatting churn.

## Definition of done

The work is complete only when all applicable points below are true:

- [ ] The existing Go API connects to the intended PostgreSQL database through a managed pool.
- [ ] Startup/shutdown and error behavior follow existing project conventions.
- [ ] The first mailbox-message read route uses the PostgreSQL repository instead of in-memory mailbox data.
- [ ] The repository uses real schema verified from migrations and Rust code.
- [ ] Authentication resolves a durable principal in production.
- [ ] Mailbox read authorization is enforced for each request.
- [ ] Cross-tenant and unauthorized mailbox access tests pass.
- [ ] Existing API route and response contracts remain compatible.
- [ ] Relevant Go tests, vet, formatting, and available integration checks have been run.
- [ ] Configuration and any schema changes are documented.
- [ ] Remaining limitations and unexecuted tests are explicitly reported.

## Required working method

Proceed in this order:

1. Inspect and summarize the relevant architecture and current implementation.
2. Identify the exact existing route and source of its in-memory data.
3. Identify the authoritative PostgreSQL schema and Rust query contract.
4. Propose a concise, minimal implementation plan.
5. Implement the first read-only vertical slice.
6. Add authentication/authorization safeguards and regression tests.
7. Run formatters and available tests.
8. Review the diff and summarize the changes.
9. Report any unresolved schema or policy decisions instead of guessing.

If working in agent mode, continue through implementation and verification without stopping after the plan, unless a critical ambiguity, destructive action, or missing required secret/permission makes safe progress impossible. Ask a concise question only when the answer materially affects correctness or security.

## Final response format

When finished, report:

1. **Root cause:** What actually caused the integration gap, based on inspected code.
2. **Files changed:** Each file and the reason for changing it.
3. **Implementation:** How the Go API now connects to PostgreSQL, resolves identity, and authorizes mailbox access.
4. **Database changes:** Exact migration(s), if any, and why they were needed.
5. **Security:** How tenant isolation, API-key scope, and mailbox permissions are enforced.
6. **Verification:** Exact commands run and pass/fail outcomes.
7. **Manual steps:** Required environment variables, migrations, or local setup.
8. **Remaining work:** Clearly identify anything incomplete or unverified.

**Important:** Do not merely explain how a developer could make these changes. Inspect the repository and make the changes directly where tool permissions and environment allow. Never claim a file was changed or a test passed unless it actually happened.
