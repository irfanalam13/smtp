#!/usr/bin/env bash
set -euo pipefail

grep -Eq '^DATABASE_URL=postgres://[^[:space:]]+$' .env.example
grep -Eq 'DATABASE_URL: postgres://[^[:space:]]+' .github/workflows/ci.yml
grep -Eq 'TEST_DATABASE_URL: postgres://[^[:space:]]+' .github/workflows/ci.yml

printf '%s\n' 'Foundation configuration references are valid.'
