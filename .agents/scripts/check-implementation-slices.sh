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

exec ruby "$ROOT/.agents/scripts/implementation-slices.rb" check
