# Security testing

Phase 07 tests cover:

- Default-deny authorization.
- Explicit permission grants.
- Cross-tenant resource denial.
- Unauthenticated API rejection.
- API-key one-time metadata behavior.
- API-key authentication and expiry/revocation paths.
- Secret-safe diagnostics and bounded request handling.

Run the focused checks from the repository root:

```sh
(cd smtp-api && gofmt -w internal/security internal/controlplane cmd/api)
(cd smtp-api && go test ./...)
(cd smtp-api && go vet ./...)
```

The repository security workflow remains authoritative for dependency and
secret scanning. Existing RustSec findings must be remediated or explicitly
reviewed; they must not be suppressed merely to obtain a green build.
