#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "Usage: .agents/scripts/implementation-review-packet.sh <slice-id> [base]"
  exit 1
fi

slice_id="$1"
base="${2:-HEAD}"
git rev-parse --verify "$base" >/dev/null

.agents/scripts/context-for-slice.sh "$slice_id"

echo
echo "# Required External Handoff"
echo
echo "Provide these author/tester inputs alongside this generated packet:"
echo
echo "- Exact user request"
echo "- Author assumptions and scope boundary"
echo "- Verifiable success criteria"
echo "- Exact verification commands and results"
echo "- UI evidence when the change is user-visible"

echo
echo "# Implementation Diff"
echo
echo "## Working tree"
git status --short

echo
echo "## Changed files against $base"
git diff --name-status "$base" -- .
git ls-files --others --exclude-standard | sed 's/^/A\t/'

echo
echo "## Diff stat"
git diff --stat "$base" -- .

echo
echo "## Rename and deletion summary"
git diff --summary --find-renames "$base" -- .

echo
echo "## Diff"
git diff --no-ext-diff "$base" -- .

while IFS= read -r file; do
  git diff --no-index -- /dev/null "$file" || true
done < <(git ls-files --others --exclude-standard)
