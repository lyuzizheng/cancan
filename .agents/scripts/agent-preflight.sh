#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

required=(
  "AGENTS.md"
  ".github/workflows/docs-harness.yml"
  "docs/README.md"
  "docs/STRUCTURE.md"
  "docs/agent/current-state.md"
  "docs/agent/reading-order.md"
  "docs/agent/iteration-protocol.md"
  "docs/agent/consistency-checklist.md"
  "docs/specs/README.md"
  ".agents/README.md"
  ".agents/ROUTER.md"
  ".agents/docs-semantic-review.md"
  ".agents/scripts/check-ci-workflow.sh"
  ".agents/scripts/docs-review-packet.sh"
  ".agents/scripts/harness-self-test.sh"
  ".agents/scripts/new-spec.sh"
)

echo "CanCan agent preflight"
echo "Repo: $ROOT"

for path in "${required[@]}"; do
  if [ ! -f "$path" ]; then
    echo "Missing required file: $path"
    exit 1
  fi
done

for script in .agents/scripts/*.sh; do
  bash -n "$script"
done

.agents/scripts/check-ci-workflow.sh
.agents/scripts/check-spec-index.sh
.agents/scripts/check-links.sh
.agents/scripts/check-docs-consistency.sh
.agents/scripts/check-agent-skills.sh

echo
echo "Read order: docs/agent/reading-order.md"

echo "Semantic review rule: required for changes under docs/, .agents/, AGENTS.md, or the docs-harness workflow"
echo "Deterministic CI does not attest semantic review."
echo "Packet: .agents/scripts/docs-review-packet.sh <base>"

echo
echo "Working tree:"
git status --short
