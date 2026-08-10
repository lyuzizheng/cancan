#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

title="${*:-}"
if [ -z "$title" ]; then
  usage "Usage: .agents/scripts/new-spec.sh \"Spec Title\""
fi

last="$(spec_files | sed -E 's#.*/([0-9]{4})-.*#\1#' | tail -1)"
if [ -n "$last" ]; then
  next_num="$(printf '%04d' "$((10#$last + 1))")"
else
  next_num="0001"
fi
slug="$(printf '%s' "$title" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-//; s/-$//')"
if [ -z "$slug" ]; then
  usage "Title must contain at least one ASCII letter or number for the filename."
fi
path="docs/specs/${next_num}-${slug}.md"

if [ -f "$path" ]; then
  die "Spec already exists: $path"
fi

cat > "$path" <<SPEC
# ${next_num}. ${title}

## Goal

## Stable decisions

## Data/API/UI behavior

## Edge cases

## Tests / acceptance criteria
SPEC

echo "Created $path"
echo "Update docs/specs/README.md, current-state/progress when relevant, and run agent-preflight before finishing."
