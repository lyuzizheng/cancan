#!/usr/bin/env bash
# Fails when the committed `Save to CanCan Inbox` Shortcut artifact or its
# manifest drifts from `scripts/shortcuts/build-save-to-cancan-inbox.py`, the
# only writer of either file.
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
require_tools python3

python3 scripts/shortcuts/build-save-to-cancan-inbox.py --check
