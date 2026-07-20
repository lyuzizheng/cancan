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
base_sha="$(git rev-parse --verify "${base}^{commit}")"
head_sha="$(git rev-parse --verify "HEAD^{commit}")"

.agents/scripts/context-for-slice.sh "$slice_id" >/dev/null

echo "# CanCan Implementation Review Handoff"
echo
echo "- Slice ID: $slice_id"
echo "- Base commit: $base_sha"
echo "- Head commit: $head_sha"
echo "- Canonical source index: run .agents/scripts/context-for-slice.sh $slice_id from this head"

echo
echo "# Required External Handoff"
echo
echo "Provide these author/tester inputs alongside this generated packet:"
echo
echo "- Exact user request"
echo "- Author assumptions and scope boundary"
echo "- Verifiable success criteria"
echo "- Selected execution tier and justification"
echo "- Canonical sources inspected at the exact head commit, listing every indexed path"
echo "- Exact verification commands and results"
echo "- UI evidence when the change is user-visible"
echo "- Previous findings and resolutions when this is a re-review"

echo
echo "# Implementation Diff"
echo
echo "## Working tree"
git status --short

echo
echo "## Changed files against $base_sha"
git diff --name-status "$base_sha" -- .
git ls-files --others --exclude-standard | sed 's/^/A\t/'

echo
echo "## Diff stat"
git diff --stat "$base_sha" -- .

echo
echo "## Rename and deletion summary"
git diff --summary --find-renames "$base_sha" -- .

echo
echo "## Repository inspection"
echo "Inspect the complete cumulative diff directly from this shared working tree."
echo "- Tracked changes: git diff --no-ext-diff $base_sha -- ."
echo "- Untracked files: open every path marked A above directly"
echo "- Re-run repository searches required by .agents/workflows/review-code.md"
