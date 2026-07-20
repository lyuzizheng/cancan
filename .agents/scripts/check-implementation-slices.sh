#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

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

if ! rg -q 'context-for-slice[.]sh' "$ROOT/.agents/scripts/implementation-review-packet.sh"; then
  echo "Implementation review packet must use the shared slice context."
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
implement_workflow="$ROOT/.agents/workflows/implement-feature.md"
review_workflow="$ROOT/.agents/workflows/review-code.md"
testing_workflow="$ROOT/.agents/workflows/simulated-testing.md"
review_packet="$ROOT/.agents/scripts/implementation-review-packet.sh"

if ! rg -q 'open the full contents of every listed source from the exact working tree and head' "$ROOT/AGENTS.md"; then
  echo "Root instructions must require full canonical-source inspection from the exact working tree and head."
  exit 1
fi

for workflow in "$implement_workflow" "$review_workflow" "$testing_workflow"; do
  if ! rg -qi 'open the full contents of every source listed by the generated index' "$workflow"; then
    echo "Implementation, review, and testing must retain full indexed-source inspection: $workflow"
    exit 1
  fi
done

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

cleanup_anchors=(
  'Reject overengineering:'
  'superseded paths and diff-created orphans'
  'package boundaries and public APIs'
  'magic logic:.*tests that hit the active path'
)
for anchor in "${cleanup_anchors[@]}"; do
  if ! rg -q "$anchor" "$review_workflow"; then
    echo "Critical cleanup gate is missing required review content: $anchor"
    exit 1
  fi
done

if ! rg -q 're-review the entire cumulative diff' "$development_cycle" ||
   ! rg -q 're-review the entire cumulative diff' "$review_workflow"; then
  echo "Testing and review must repeat over the entire cumulative diff after fixes."
  exit 1
fi

if ! rg -q 'Do not duplicate the root transcript or tool history' "$development_cycle"; then
  echo "Independent roles must receive a compact handoff instead of the root transcript."
  exit 1
fi

if ! rg -q 'canonical sources inspected at the exact head commit' "$development_cycle"; then
  echo "Independent role handoffs must record canonical-source inspection at the exact head."
  exit 1
fi

if ! rg -q 'inspect the complete cumulative diff directly from the shared repository' "$review_workflow"; then
  echo "Review workflow must inspect the complete cumulative diff from the shared repository."
  exit 1
fi

packet_anchors=(
  '^echo "# CanCan Implementation Review Handoff"$'
  'Slice ID: [$]slice_id'
  'Base commit: [$]base_sha'
  'Head commit: [$]head_sha'
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
  'Inspect the complete cumulative diff directly from this shared working tree'
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

if rg -q '^git diff --no-ext-diff "[$]base_sha" -- [.]$' "$review_packet"; then
  echo "Implementation review packet must not copy the full cumulative diff into the handoff."
  exit 1
fi

exec ruby "$ROOT/.agents/scripts/implementation-slices.rb" check
