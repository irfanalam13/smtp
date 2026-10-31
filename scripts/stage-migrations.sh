#!/usr/bin/env bash
set -euo pipefail

destination=${1:?Usage: stage-migrations.sh DESTINATION}
mkdir -p "$destination"

if find "$destination" -mindepth 1 -print -quit | grep -q .; then
  printf 'Migration staging directory must be empty: %s\n' "$destination" >&2
  exit 1
fi

while IFS= read -r -d '' migration; do
  cp "$migration" "$destination/"
done < <(find smtp-storage/postgres -mindepth 2 -maxdepth 2 -type f -name '*.sql' -print0 | sort -z)
