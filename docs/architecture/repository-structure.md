# Repository structure

The top-level modules use one canonical directory per platform boundary. The
Rust workspace currently contains `smtp-core`, `smtp-session`, `smtp-queue`, and
`smtp-delivery`; the Go control plane is `smtp-api`; and the Next.js application
is `smtp-dashboard`.

| Directory | Ownership |
| --- | --- |
| `smtp-core` | Shared Rust contracts and domain types |
| `smtp-parser` | Future reusable SMTP parsing boundary |
| `smtp-session` | SMTP listener, protocol state machine, and inbound acceptance |
| `smtp-auth` | Future authentication mechanisms and policy |
| `smtp-delivery` | Outbound SMTP transport |
| `smtp-queue` | Durable queue and worker |
| `smtp-storage` | PostgreSQL migrations and storage assets |
| `smtp-imap`, `smtp-pop3` | Future mailbox protocol services |
| `smtp-api` | Go control-plane API |
| `smtp-dashboard` | Next.js operator dashboard |
| `smtp-analytics`, `smtp-ai` | Future analytics and AI boundaries |
| `smtp-security`, `smtp-monitoring` | Cross-cutting security and observability |
| `smtp-kubernetes` | Future Kubernetes-specific assets |
| `smtp-sdk`, `smtp-cli` | Future client SDKs and operator CLI |
| `smtp-tests` | Integration and end-to-end tests |
| `docs` | Architecture, API, development, and security documentation |
| `infrastructure` | Environment-independent deployment assets |

Unimplemented boundaries contain documentation only. New functionality should
be added to the owning module rather than duplicated in a second top-level
directory.
