# Repository structure

This repository is a monorepo with explicit language and service boundaries.
Keep a capability in the component that owns its runtime and data contract;
split files by responsibility within that component instead of creating
parallel implementations or a catch-all module.

## Workspace boundaries

| Path | Boundary and responsibility |
|---|---|
| `smtp-core/` | Rust shared contracts and domain logic: configuration, identity/context, events, authentication, MIME, mailbox storage and workflows, search, attachments, synchronization, and shared runtime infrastructure. |
| `smtp-session/` | Rust SMTP listener, command/session handling, inbound acceptance, and protocol responses. |
| `smtp-queue/` | Rust durable queue models, storage, retry, processing, and worker lifecycle. |
| `smtp-delivery/` | Rust outbound SMTP transport, DNS, framing, responses, and delivery outcomes. |
| `smtp-imap/`, `smtp-pop3/` | Rust protocol workspace boundaries for mailbox access; keep protocol-specific behavior here rather than in the API or shared core. |
| `smtp-api/` | Independent Go module. `cmd/` contains executables; `internal/` owns the control plane and cross-cutting API packages. |
| `smtp-dashboard/` | Next.js application and the root pnpm workspace member. |
| `smtp-storage/postgres/` | Feature-grouped, globally ordered PostgreSQL migration pairs. Schema changes belong here. |
| `docs/` | API contracts, architecture, development, operations, security, standards, and frontend conventions. |
| `scripts/`, `infrastructure/`, `observability/`, `.github/` | Repository tooling, deployment assets, operational configuration, and CI workflows. |

The Rust workspace is declared in the root `Cargo.toml`; the Go module is
declared in `smtp-api/go.mod`; the frontend workspace is declared by the root
`pnpm-workspace.yaml`. These are separate build boundaries even though their
code is stored together.

## Feature organization inside components

### Rust shared core

`smtp-core/src/` keeps cohesive capabilities in named modules. The mailbox
store facade and shared types remain in `mailbox_store.rs`; database workflows
are grouped in the private `mailbox_store/` modules:

| Module | Responsibility |
|---|---|
| `administration.rs` | Mailbox creation, transitions, aliases, permissions, and folders |
| `messages.rs` | Message append/list, incremental changes, and quota |
| `rules.rs`, `labels.rs` | Mailbox rules and labels |
| `retention.rs`, `flags.rs` | Quarantine, trash, drafts, archive policy, flags, and expunge |
| `submissions.rs` | Sent-submission records and recipient events |
| `outbox.rs` | Outbox claiming, acknowledgement, and retry |

The submodules extend `MailboxStore` without changing the public exports from
`smtp-core`. Keep new mailbox workflows in the narrowest matching module.
Other `smtp-core/src/*.rs` files are already named by capability; do not move
them into folders mechanically.

The other Rust crates keep protocol, queue, and transport responsibilities in
their own `src/` trees. Shared contracts belong in `smtp-core`, not copied
between services.

### Go control plane

`smtp-api/internal/` keeps packages separated by boundary: `controlplane`,
`health`, `identity`, `observability`, `security`, and `webhooks`. Within
`internal/controlplane/`, files are grouped by API or persistence feature:

- `routes.go` composes routes and request context; `*_handlers.go` files own
  tenant, domain, mailbox, and API-key HTTP workflows.
- `types.go`, `errors.go`, `store.go`, and `auth.go` own shared control-plane
  models, errors, in-memory state, and authorization.
- `domains.go`, `mailboxes.go`, `api_keys.go`, and `usage.go` own in-memory
  feature behavior; `http_json.go` owns shared JSON responses and decoding.
- `postgres.go` owns repository setup and lifecycle;
  `auth_postgres.go`, `mailboxes_postgres.go`, and `api_keys_postgres.go` own
  their respective database operations. Domain and submission persistence
  remain in `domains_postgres.go` and `submissions_postgres.go`.
- `submission.go` owns request validation and submission behavior.

Keep related tests beside the package they exercise. Add a new file when a
responsibility grows; avoid restoring a single large handler, repository, or
control-plane source file.

### Dashboard

`smtp-dashboard/src/app/` owns Next.js routes and layout. Product capabilities
belong under `src/features/<feature>/`; shared UI primitives belong under
`src/components/`, and generic browser/API infrastructure belongs under
`src/lib/`. The implemented control-plane dashboard is organized into
components, hooks, services, and types. Other feature and test folders marked
with `.gitkeep` are empty scaffolds only, not implemented product features or
test runners. See the [frontend architecture](../frontend/architecture.md)
before adding dashboard code.

### Database, contracts, tests, and docs

- `smtp-storage/postgres/` owns numbered migrations grouped by schema feature,
  with matching `.up.sql` and `.down.sql` files. Their numeric prefixes define
  one global order. `scripts/stage-migrations.sh` flattens them for the
  `migrate` CLI. Do not edit an old migration to introduce a new schema change;
  add the next migration pair to its feature folder.
- `docs/api/openapi.yaml` is the HTTP contract. Keep API routes, handlers,
  dashboard service calls, and OpenAPI changes aligned.
- Rust unit/integration tests live with their crate; Go tests live with their
  package. `smtp-tests/` is a separate integration-test boundary and should
  contain only tests that need cross-component setup.
- `docs/architecture/` records system and repository boundaries;
  `docs/frontend/` records dashboard-specific structure and conventions.

## Planned boundaries

Top-level directories such as `smtp-ai/`, `smtp-analytics/`, `smtp-auth/`,
`smtp-cli/`, `smtp-kubernetes/`, `smtp-monitoring/`, `smtp-parser/`,
`smtp-sdk/`, and `smtp-security/` may contain design or README material
without being implemented workspaces. Treat documentation as intent, not
proof that a service or feature exists. Do not move code into these directories
or add placeholder implementations until the capability has an owner and a
real runtime or package boundary.

## Change checklist

1. Put the implementation in the owning workspace and feature module.
2. Update the public contract and callers when a boundary changes.
3. Keep migrations with the PostgreSQL storage owner and preserve their order.
4. Add or update tests in the owning crate or package.
5. Update the relevant architecture or feature documentation.
6. Run the smallest focused checks, then the workspace checks required by the
   changed boundaries.
