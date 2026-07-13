#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

required=(
  "AGENTS.md"
  ".github/workflows/docs-harness.yml"
  ".github/workflows/application.yml"
  "docs/README.md"
  "docs/STRUCTURE.md"
  "docs/agent/current-state.md"
  "docs/agent/reading-order.md"
  "docs/agent/iteration-protocol.md"
  "docs/agent/consistency-checklist.md"
  "docs/agent/implementation-slices.md"
  "docs/specs/README.md"
  ".agents/README.md"
  ".agents/ROUTER.md"
  ".agents/docs-semantic-review.md"
  ".agents/scripts/check-ci-workflow.sh"
  ".agents/scripts/check-implementation-slices.sh"
  ".agents/scripts/context-for-slice.sh"
  ".agents/scripts/docs-review-packet.sh"
  ".agents/scripts/harness-self-test.sh"
  ".agents/scripts/implementation-review-packet.sh"
  ".agents/scripts/implementation-slices.rb"
  ".agents/scripts/new-spec.sh"
  ".node-version"
  "rust-toolchain.toml"
  "scripts/dev-toolchain.env"
  "scripts/setup-dev.sh"
  "scripts/test-setup-dev.sh"
  "package.json"
  "pnpm-lock.yaml"
  "pnpm-workspace.yaml"
  "tsconfig.base.json"
  "apps/desktop/package.json"
  "apps/desktop/src-tauri/Cargo.lock"
  "apps/desktop/src-tauri/Cargo.toml"
  "apps/desktop/src-tauri/tauri.conf.json"
  "packages/core/package.json"
  "packages/db/package.json"
  "packages/ui/package.json"
  "spikes/desktop-feasibility/package.json"
  "spikes/desktop-feasibility/scripts/verify.sh"
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

for script in scripts/*.sh; do
  bash -n "$script"
done

for script in .agents/scripts/*.rb; do
  ruby -c "$script" >/dev/null
done

.agents/scripts/check-ci-workflow.sh
.agents/scripts/check-spec-index.sh
.agents/scripts/check-links.sh
.agents/scripts/check-implementation-slices.sh
.agents/scripts/check-docs-consistency.sh
.agents/scripts/check-agent-skills.sh
scripts/test-setup-dev.sh

echo
echo "Read order: docs/agent/reading-order.md"

echo "Semantic review rule: required for changes under docs/, .agents/, AGENTS.md, or the docs-harness workflow"
echo "Deterministic CI does not attest semantic review."
echo "Packet: .agents/scripts/docs-review-packet.sh <base>"
echo "Slices: .agents/scripts/implementation-slices.rb list"
echo "Slice context: .agents/scripts/context-for-slice.sh <slice-id>"
echo "Implementation review: .agents/scripts/implementation-review-packet.sh <slice-id> [base]"

echo
echo "Working tree:"
git status --short
