#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

required=(
  "docs/README.md"
  "docs/STRUCTURE.md"
  "docs/agent/current-state.md"
  "docs/agent/reading-order.md"
  "docs/agent/iteration-protocol.md"
  "docs/agent/consistency-checklist.md"
  "docs/specs/README.md"
  ".agents/README.md"
  ".agents/ROUTER.md"
)

echo "CanCan agent preflight"
echo "Repo: $ROOT"

for path in "${required[@]}"; do
  if [ ! -f "$path" ]; then
    echo "Missing required file: $path"
    exit 1
  fi
done

.agents/scripts/check-docs-consistency.sh
.agents/scripts/check-agent-skills.sh

echo
echo "Read order:"
printf '  %s\n' "${required[@]}"

echo
echo "Working tree:"
git status --short
