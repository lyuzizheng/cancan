#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
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

echo "Checking harness does not own current priorities..."
if rg -n '^## Current Priorit(y|ies)' .agents --glob '*.md'; then
  echo "Current priorities belong only in current-state/alignment docs."
  fail=1
fi

echo "Checking removed harness layers are not referenced..."
removed_refs='\.agents/(roles|rules|plugins|templates)/|role-for-prompt[.]sh|docs/alignment-temp/(grill-backlog|lifecycle-breakdown|doc-consistency-audit)[.]md'
if rg -n "$removed_refs" AGENTS.md README.md docs .agents --glob '*.md'; then
  echo "Removed harness layer is still referenced."
  fail=1
fi

echo "Checking backticked repo file references..."
while IFS= read -r entry; do
  reference="${entry##*:}"
  reference="${reference#\`}"
  reference="${reference%\`}"
  case "$reference" in
    *'*'*) continue ;;
  esac
  if [ ! -e "$reference" ]; then
    echo "Missing repo file reference: $entry"
    fail=1
  fi
done < <((rg -n -o '`(docs|[.]agents)/[^`]+[.](md|sh)`' AGENTS.md README.md docs .agents --glob '*.md' || true))

echo "Checking ADR statuses..."
for adr in docs/adr/*.md; do
  status="$(awk '/^## Status$/{getline; while ($0 == "") getline; print; exit}' "$adr")"
  case "$status" in
    Proposed|Accepted|Superseded|Deprecated|Rejected) ;;
    *)
      echo "Invalid or missing ADR status in $adr: ${status:-<empty>}"
      fail=1
      ;;
  esac
done

echo "Checking whitespace errors..."
if rg -n '[[:blank:]]+$' AGENTS.md README.md .gitignore docs .agents .github --glob '*.md' --glob '*.sh' --glob '*.yml' --glob '*.yaml' --glob '.gitignore'; then
  echo "Trailing whitespace found in the current working tree."
  fail=1
fi
if ! git diff --check; then
  fail=1
fi
if ! git diff --cached --check; then
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "Doc consistency check failed."
  exit 1
fi

echo "Doc consistency check passed."
