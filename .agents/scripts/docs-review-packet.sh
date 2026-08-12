#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

base="${1:-HEAD}"
git rev-parse --verify "$base" >/dev/null

cat .agents/docs-semantic-review.md

echo
echo "## Changed files against $base"
git diff --name-status "$base" -- AGENTS.md .github/workflows/docs-harness.yml docs .agents .codex
git ls-files --others --exclude-standard -- AGENTS.md .github/workflows/docs-harness.yml docs .agents .codex | sed 's/^/A\t/'

echo
echo "## Diff"
git diff --no-ext-diff "$base" -- AGENTS.md .github/workflows/docs-harness.yml docs .agents .codex

while IFS= read -r file; do
  git diff --no-index -- /dev/null "$file" || true
done < <(git ls-files --others --exclude-standard -- AGENTS.md .github/workflows/docs-harness.yml docs .agents .codex)
