#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
require_tools ruby

if [ "$#" -ne 1 ]; then
  usage "Usage: .agents/scripts/context-for-slice.sh <slice-id>"
fi

exec ruby "$ROOT/.agents/scripts/implementation-slices.rb" context "$1"
