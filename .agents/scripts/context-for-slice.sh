#!/usr/bin/env bash
set -euo pipefail

ROOT="${CANCAN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

if [ "$#" -ne 1 ]; then
  echo "Usage: .agents/scripts/context-for-slice.sh <slice-id>"
  exit 1
fi

exec ruby "$ROOT/.agents/scripts/implementation-slices.rb" context "$1"
