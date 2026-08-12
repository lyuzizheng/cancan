#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
require_tools rg ruby

# This gate validates structural wiring, required headings, and the review-packet
# generator's own output contract. It deliberately does NOT grep for exact English
# sentences in human-authored docs (AGENTS.md, .agents/workflows/*.md): those
# behavioral rules have one canonical home and are reviewed semantically, not by
# string match. Coupling the gate to prose made benign rewording break CI.

required=(
  ".agents/scripts/implementation-slices.rb"
  ".agents/scripts/context-for-slice.sh"
  ".agents/scripts/implementation-review-packet.sh"
  "docs/agent/implementation-slices.md"
)

for path in "${required[@]}"; do
  if [ ! -f "$ROOT/$path" ]; then
    echo "Missing implementation harness file: $path"
    exit 1
  fi
done

for path in .agents/scripts/implementation-slices.rb .agents/scripts/context-for-slice.sh .agents/scripts/implementation-review-packet.sh; do
  if [ ! -x "$ROOT/$path" ]; then
    echo "Implementation harness script is not executable: $path"
    exit 1
  fi
done

if ! rg -q '^[.]agents/scripts/context-for-slice[.]sh "[$]slice_id" >/dev/null$' "$ROOT/.agents/scripts/implementation-review-packet.sh"; then
  echo "Implementation review packet must validate the selected slice with the shared context generator."
  exit 1
fi

if rg -q 'puts File[.]read' "$ROOT/.agents/scripts/implementation-slices.rb"; then
  echo "Implementation context must index canonical sources instead of copying their full content."
  exit 1
fi

if ! rg -q 'context-for-slice[.]sh' "$ROOT/.agents/workflows/implement-feature.md"; then
  echo "Implement workflow must use the shared slice context."
  exit 1
fi

if ! rg -q 'implementation-review-packet[.]sh' "$ROOT/.agents/workflows/review-code.md"; then
  echo "Review workflow must use the shared implementation packet."
  exit 1
fi

if ! rg -q 'context-for-slice[.]sh' "$ROOT/.agents/workflows/simulated-testing.md"; then
  echo "Testing workflow must use the shared slice context."
  exit 1
fi

development_cycle="$ROOT/.agents/workflows/development-cycle.md"
review_workflow="$ROOT/.agents/workflows/review-code.md"
review_packet="$ROOT/.agents/scripts/implementation-review-packet.sh"

if ! rg -q '[.]agents/scripts/agent-preflight[.]sh' "$development_cycle"; then
  echo "Development cycle must retain the repository preflight."
  exit 1
fi

if ! rg -q '`pnpm verify`' "$development_cycle"; then
  echo "Development cycle must retain the application verification command."
  exit 1
fi

cleanup_gate_count="$(rg -c '^## Critical cleanup gate$' "$review_workflow" || true)"
if [ "${cleanup_gate_count:-0}" -ne 1 ]; then
  echo "Review workflow must contain exactly one Critical cleanup gate."
  exit 1
fi

packet_anchors=(
  '^echo "# CanCan Implementation Review Handoff"$'
  'Slice ID: [$]slice_id'
  'Base commit: [$]base_sha'
  'Head commit: [$]head_sha'
  'Working tree fingerprint: [$]worktree_fingerprint'
  'Canonical source index: run [. ]*agents/scripts/context-for-slice[.]sh [$]slice_id from this head'
  '^echo "# Required External Handoff"$'
  'Exact user request'
  'Author assumptions and scope boundary'
  'Verifiable success criteria'
  'Selected execution tier and justification'
  'Canonical sources inspected at the exact head commit'
  'Exact verification commands and results'
  'UI evidence when the change is user-visible'
  'Previous findings and resolutions'
  '^echo "## Diff stat"$'
  'git diff --stat "[$]base_sha" -- [.]'
  '^echo "## Rename and deletion summary"$'
  'git diff --summary --find-renames "[$]base_sha" -- [.]'
  '^echo "## Repository inspection"$'
  'HEAD check: test.*git rev-parse --verify HEAD'
  'Require the refreshed Head commit and Working tree fingerprint to match this packet'
  'Inspect the complete cumulative diff directly from the verified shared working tree'
  'git diff --no-ext-diff [$]base_sha [$]head_sha -- [.]'
  'git diff --no-ext-diff [$]head_sha -- [.]'
)
for anchor in "${packet_anchors[@]}"; do
  if ! rg -q "$anchor" "$review_packet"; then
    echo "Implementation review packet is missing required evidence: $anchor"
    exit 1
  fi
done

if rg -q '^[.]agents/scripts/context-for-slice[.]sh "[$]slice_id"$' "$review_packet"; then
  echo "Implementation review packet must not embed the generated source index."
  exit 1
fi

if rg -q '^[[:space:]]*git diff --no-ext-diff ' "$review_packet"; then
  echo "Implementation review packet must not copy the full cumulative diff into the handoff."
  exit 1
fi

exec ruby "$ROOT/.agents/scripts/implementation-slices.rb" check
