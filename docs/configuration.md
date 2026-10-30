# Configuration reference

Configuration is supplied through environment variables. `.env` is for local
development only and must not be committed or reused as a production secret
file.

## Local configuration

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `POSTGRES_PASSWORD` | Yes | None | Compose development database password |
| `DATABASE_URL` | Queue/integration use | Derived locally | PostgreSQL connection URL |
| `API_ADDR` | No | `127.0.0.1:8080` | Go API listen address; the local runner rejects non-loopback addresses |
| `API_URL` | No | `http://127.0.0.1:8080` | Backend origin used by the Next.js dashboard rewrite |
| `CONTROL_PLANE_DEV_MODE` | No | `false` | Enables the local `X-Dev-Subject` principal adapter |
| `OIDC_ISSUER_URL` | Optional | Unset | Keycloak realm issuer URL; configure together with `OIDC_API_AUDIENCE` |
| `OIDC_API_AUDIENCE` | Optional | Unset | Expected audience for Keycloak bearer access tokens |
| `SMTP_DELIVERY_ENABLED` | No | `false` | Enables external SMTP delivery only when explicitly configured |
| `SMTP_DELIVERY_ALLOWLIST` | Delivery use | None | Required safety boundary for enabled external delivery |
| `TEST_DATABASE_URL` | Queue tests | None | Isolated disposable PostgreSQL database for ignored integration tests |

## Safety rules

- Keep `API_ADDR`, PostgreSQL, and SMTP loopback-bound during development.
- Do not set `CONTROL_PLANE_DEV_MODE=true` in deployment.
- Do not enable external delivery without a reviewed destination policy.
- Do not point `TEST_DATABASE_URL` at a database containing useful data.
- Store production secrets in a secret manager, not `.env`.
- Configure both OIDC variables or neither. Production issuer URLs must use
  HTTPS; HTTP is accepted only for loopback development issuers.
- Use a dedicated API audience in Keycloak access tokens; do not use an ID
  token as an API bearer token.

## Configuration ownership

The Go API currently reads `API_ADDR`, `CONTROL_PLANE_DEV_MODE`, and the
optional Keycloak OIDC issuer/audience pair.
The SMTP ingress and queue read `DATABASE_URL` through their Rust
configuration. Delivery settings are owned by `mail-delivery`. The
development scripts export shared values before starting processes.
