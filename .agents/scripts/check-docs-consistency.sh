#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

fail=0

echo "Checking removed numbered-doc layer..."
if find docs -maxdepth 1 -type f -regex '.*/[0-9][0-9]-.*[.]md' | grep -q .; then
  echo "Unexpected numbered docs found under docs/."
  find docs -maxdepth 1 -type f -regex '.*/[0-9][0-9]-.*[.]md'
  fail=1
fi

echo "Checking private fixtures are not tracked..."
if git ls-files 'fixtures-private/*' | grep -q .; then
  echo "Tracked files found under fixtures-private/. These must remain local-only."
  git ls-files 'fixtures-private/*'
  fail=1
fi

echo "Checking stale design claims..."
patterns=(
  "Default base currency is SGD"
  "base-currency calculation"
  "base-currency value"
  "net worth in base currency"
  "Top: net worth"
  "stale currency rates"
  "stale price"
  "without broker API"
  "current market values"
  "net worth confidence"
  "update numbered docs"
  "moved into numbered"
  "permanent docs/specs"
)

for pattern in "${patterns[@]}"; do
  if rg -n -i "$pattern" docs .agents --glob '*.md' >/tmp/cancan-doc-check.out; then
    echo "Stale pattern found: $pattern"
    cat /tmp/cancan-doc-check.out
    fail=1
  fi
done

rm -f /tmp/cancan-doc-check.out

echo "Checking whitespace errors..."
if ! git diff --check; then
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "Doc consistency check failed."
  exit 1
fi

echo "Doc consistency check passed."
