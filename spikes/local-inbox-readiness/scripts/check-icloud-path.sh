#!/usr/bin/env bash
set -euo pipefail

ICLOUD_DRIVE_PATH="$HOME/Library/Mobile Documents/com~apple~CloudDocs"

if [[ -d "$ICLOUD_DRIVE_PATH" && -r "$ICLOUD_DRIVE_PATH" ]]; then
  printf 'iCloud Drive path is readable; live iCloud evidence still requires a dedicated run.\n'
else
  printf 'iCloud Drive path is absent or unreadable; live iCloud evidence remains blocked.\n'
fi
