.PHONY: help setup doctor infra-up infra-down infra-ps dev-bootstrap dev-up dev-run deploy-preflight dev-down fmt fmt-check lint test test-api test-queue test-delivery build check clean migrate-up migrate-down migrate-validate backup-postgres verify-postgres-backup validate-foundation-config

help:
	@printf '%s\n' \
		'Usage: make <target>' \
		'' \
		'Environment:' \
		'  setup          Check prerequisites and bootstrap local dependencies' \
		'  doctor         Report required tool versions and repository state' \
		'  infra-up       Start PostgreSQL and wait for readiness' \
		'  infra-down     Stop local infrastructure without deleting data' \
		'  infra-ps       Show local infrastructure status and health' \
		'' \
		'Validation:' \
		'  fmt            Format Rust, Go, and dashboard sources' \
		'  fmt-check      Check formatting without changing files' \
		'  lint           Run Rust, Go, and dashboard linters' \
		'  test           Run Rust and Go tests' \
		'  build          Build Rust, Go, and dashboard artifacts' \
		'  check          Run the essential repository validation suite' \
		'  clean          Remove generated build artifacts only' \
		'' \
		'Legacy/ focused:' \
		'  dev-bootstrap   Alias for setup' \
		'  dev-run         Start the local application processes' \
		'  test-api        Run Go API tests' \
		'  test-queue      Run PostgreSQL queue integration tests' \
		'  test-delivery   Run delivery crate tests' \
		'  migrate-validate Validate migrations against DATABASE_URL' \
		'  backup-postgres Create a logical PostgreSQL backup' \
		'  verify-postgres-backup Verify a backup SHA-256 sidecar' \
		'  validate-foundation-config Check local and CI configuration references'

setup:
	./scripts/bootstrap-dev.sh

doctor:
	@set -eu; \
	printf '%s\n' 'Development environment doctor'; \
	missing=0; \
	for command in cargo rustc rustup go pnpm docker make openssl; do \
		if command -v "$$command" >/dev/null 2>&1; then \
			printf '  %-8s ' "$$command"; \
			if [ "$$command" = go ]; then "$$command" version; \
			else "$$command" --version 2>/dev/null | head -n 1; fi; \
		else \
			printf '  %-8s MISSING\n' "$$command"; missing=1; \
		fi; \
	done; \
	if docker compose version >/dev/null 2>&1; then \
		printf '  %-8s %s\n' compose "$$(docker compose version --short)"; \
	else \
		printf '  compose  MISSING (Docker Compose v2 required)\n'; missing=1; \
	fi; \
	test "$$missing" -eq 0 || { printf '%s\n' 'Install the missing prerequisites and rerun make doctor.' >&2; exit 1; }; \
	test -f Cargo.toml || { printf 'Missing root Cargo.toml\n' >&2; exit 1; }; \
	test -f .env.example || { printf 'Missing .env.example\n' >&2; exit 1; }; \
	printf '  repository root: %s\n' "$$(pwd)"

infra-up:
	@test -f .env || { printf 'Missing .env. Run: cp .env.example .env\n' >&2; exit 1; }
	docker compose up -d postgres
	@for attempt in $$(seq 1 30); do \
		if docker compose exec -T postgres pg_isready -U smtp_dev -d smtp_platform >/dev/null 2>&1; then \
			printf '%s\n' 'PostgreSQL is ready.'; exit 0; \
		fi; \
		if [ "$$attempt" -eq 30 ]; then \
			printf '%s\n' 'PostgreSQL did not become ready.' >&2; exit 1; \
		fi; \
		sleep 1; \
	done

infra-down:
	docker compose down

infra-ps:
	docker compose ps

dev-bootstrap:
	$(MAKE) setup

dev-up:
	$(MAKE) infra-up

dev-run:
	./scripts/run-dev.sh

deploy-preflight:
	./scripts/deploy-preflight.sh --production-env

dev-down:
	docker compose down

fmt:
	cargo fmt --all
	cd smtp-api && gofmt -w .
	pnpm exec prettier --write smtp-dashboard

fmt-check:
	cargo fmt --all -- --check
	test -z "$$(gofmt -l smtp-api)" || { printf 'Go files need formatting:\n%s\n' "$$(gofmt -l smtp-api)" >&2; exit 1; }
	pnpm exec prettier --check smtp-dashboard

lint:
	cargo clippy --workspace --all-targets -- -D warnings
	cd smtp-api && go vet ./...
	pnpm lint:dashboard

test:
	cargo test --workspace --all-targets
	cd smtp-api && go test ./...

test-queue:
	@test -n "$${TEST_DATABASE_URL}" || (echo "TEST_DATABASE_URL must point to an isolated disposable database" >&2; exit 1)
	cargo test -p mail-queue --test postgres -- --ignored --test-threads=1

test-api:
	cd smtp-api && go test ./...

test-delivery:
	cargo test -p mail-delivery

build:
	cargo build --workspace --all-targets
	cd smtp-api && go build ./...
	pnpm build:dashboard

clean:
	cargo clean
	rm -rf smtp-dashboard/.next smtp-dashboard/tsconfig.tsbuildinfo

migrate-up:
	docker run --rm --network host -v "$(PWD)/smtp-storage/postgres:/migrations:ro" migrate/migrate:v4.18.3 \
		-path=/migrations -database "$${DATABASE_URL:?Set DATABASE_URL}" up

migrate-down:
	docker run --rm --network host -v "$(PWD)/smtp-storage/postgres:/migrations:ro" migrate/migrate:v4.18.3 \
		-path=/migrations -database "$${DATABASE_URL:?Set DATABASE_URL}" down 1

migrate-validate:
	./scripts/validate-migrations.sh

backup-postgres:
	./scripts/backup-postgres.sh

verify-postgres-backup:
	./scripts/verify-postgres-backup.sh "$${BACKUP:?Set BACKUP to a backup dump path}"

validate-foundation-config:
	./scripts/validate-foundation-config.sh

check:
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test
