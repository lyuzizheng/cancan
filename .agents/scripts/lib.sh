#!/usr/bin/env bash
# Shared helpers for CanCan .agents/scripts/*.sh deterministic gates.
#
# Source this immediately after `set -euo pipefail`:
#   source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
#
# It resolves ROOT once (honoring $CANCAN_ROOT, used by the harness self-test),
# changes into it so every gate runs from the repo root, and provides small
# helpers so the gates share one dependency declaration and one exit-code
# convention: usage error = 2, gate/tool failure = 1.

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT"

# Fail a gate (exit 1).
die() {
  echo "$@" >&2
  exit 1
}

# Reject bad invocation (exit 2).
usage() {
  echo "$@" >&2
  exit 2
}

# Verify each named executable is on PATH; exit 1 listing every missing one.
# Centralizes the dependency guard that was previously ad hoc (only `ruby`
# was ever checked, and `rg` was an undeclared hard dependency).
require_tools() {
  local tool missing=0
  for tool in "$@"; do
    command -v "$tool" >/dev/null 2>&1 || { echo "Missing required tool: $tool" >&2; missing=1; }
  done
  [ "$missing" -eq 0 ] || exit 1
}

# Sorted list of canonical spec file paths (docs/specs/NNNN-*.md).
spec_files() {
  find docs/specs -maxdepth 1 -type f -name '[0-9][0-9][0-9][0-9]-*.md' | sort
}
