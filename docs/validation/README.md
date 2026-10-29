# Phase 10 platform foundation validation

This directory records executable evidence for Phase 10. Results are not
production-capacity claims and do not certify the eventual email platform.

## Reproduce the validation

Fast local checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo build --workspace --all-targets
cargo test --workspace --doc
(cd smtp-api && gofmt -l . && go test ./... && go test -race ./... && go vet ./... && go build ./...)
python3 - <<'PY'
import json
from pathlib import Path
json.loads(Path("observability/grafana/api-overview.json").read_text())
print("dashboard JSON valid")
PY
docker compose config --quiet
```

Database validation requires an isolated disposable PostgreSQL instance:

```sh
export DATABASE_URL='postgres://smtp_dev:<test-password>@127.0.0.1:<port>/smtp_platform_test?sslmode=disable'
export TEST_DATABASE_URL="$DATABASE_URL"
./scripts/validate-migrations.sh
cargo test -p mail-queue --test postgres -- --ignored --test-threads=1
./scripts/backup-postgres.sh backups/validation.dump
pg_restore --list backups/validation.dump
```

Restore the dump only into a new disposable database as described in
[`../storage/backup-recovery.md`](../storage/backup-recovery.md). Never use a
production database or credentials for validation.

## Status conventions

- **PASS:** Executed and evidence satisfied the checked requirement.
- **FAIL:** Executed and the observed result did not satisfy the requirement.
- **BLOCKED:** The required infrastructure or tool was unavailable.
- **NOT TESTED:** No evidence was collected.
- **NOT APPLICABLE:** The requirement does not apply to the current
  implementation, with a reason recorded.

## Evidence summary

See [requirements-to-evidence.md](requirements-to-evidence.md),
[performance-baseline.md](performance-baseline.md),
[risk-register.md](risk-register.md), and
[release-readiness.md](release-readiness.md).
