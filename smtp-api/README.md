# SMTP API

This Go module contains the HTTP control plane and its supporting services.
Each directory under `internal/` is a Go package boundary. Files that share a
package stay together so handlers and repositories can use their private types
and helpers without widening the API between packages.

```text
smtp-api/
├── cmd/
│   ├── api/                 # HTTP API executable and startup wiring
│   └── bootstrap-key/       # API-key provisioning executable
└── internal/
    ├── controlplane/        # Routes, handlers, domain logic, stores, and SQL repositories
    ├── health/              # Liveness and readiness handlers
    ├── identity/            # External identity-provider verification
    ├── observability/       # Metrics and instrumentation
    ├── security/            # Principal and authorization rules
    └── webhooks/            # Webhook signatures
```

Within `controlplane/`, filenames start with the feature and use a consistent
role suffix: `*_handlers.go` for HTTP handlers, `*_postgres.go` for durable
repositories, and `*_test.go` for package tests. `routes.go` registers routes,
`types.go` and `errors.go` define shared contracts, while `store.go` and
`postgres.go` own the in-memory and PostgreSQL repository foundations.

Run the API from the repository root with `go run ./smtp-api/cmd/api`.
