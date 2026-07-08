#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

title="${*:-}"
if [ -z "$title" ]; then
  echo "Usage: .agents/scripts/new-spec.sh \"Spec Title\""
  exit 2
fi

last="$(find docs/specs -maxdepth 1 -type f -name '[0-9][0-9][0-9][0-9]-*.md' | sed -E 's#.*docs/specs/([0-9]{4})-.*#\1#' | sort | tail -1)"
next_num="$(printf '%04d' "$((10#$last + 1))")"
slug="$(printf '%s' "$title" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-//; s/-$//')"
path="docs/specs/${next_num}-${slug}.md"

if [ -f "$path" ]; then
  echo "Spec already exists: $path"
  exit 1
fi

cat > "$path" <<SPEC
# ${next_num}. ${title}

## Goal

## Stable Decisions

## Data/API/UI Behavior

## Edge Cases

## Tests / Acceptance Criteria
SPEC

echo "Created $path"
echo "Update docs/specs/README.md and docs/agent/progress-log.md before finishing."
