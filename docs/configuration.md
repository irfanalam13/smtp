# Configuration reference

Configuration is supplied through environment variables. `.env` is for local
development only and must not be committed or reused as a production secret
file.

## Local configuration

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `POSTGRES_PASSWORD` | Yes | None | Compose development database password |
| `DATABASE_URL` | Queue/integration use | Derived locally | PostgreSQL connection URL |
| `API_ADDR` | No | `:8080` | Go API listen address |
| `CONTROL_PLANE_DEV_MODE` | No | `false` | Enables the local `X-Dev-Subject` principal adapter |
| `SMTP_DELIVERY_ENABLED` | No | `false` | Enables external SMTP delivery only when explicitly configured |
| `SMTP_DELIVERY_ALLOWLIST` | Delivery use | None | Required safety boundary for enabled external delivery |
| `TEST_DATABASE_URL` | Queue tests | None | Isolated disposable PostgreSQL database for ignored integration tests |

## Safety rules

- Keep `API_ADDR`, PostgreSQL, and SMTP loopback-bound during development.
- Do not set `CONTROL_PLANE_DEV_MODE=true` in deployment.
- Do not enable external delivery without a reviewed destination policy.
- Do not point `TEST_DATABASE_URL` at a database containing useful data.
- Store production secrets in a secret manager, not `.env`.

## Configuration ownership

The Go API currently reads `API_ADDR` and `CONTROL_PLANE_DEV_MODE`.
The SMTP ingress and queue read `DATABASE_URL` through their Rust
configuration. Delivery settings are owned by `mail-delivery`. The
development scripts export shared values before starting processes.
