#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

for command in cargo pnpm; do
  if ! command -v "$command" >/dev/null; then
    echo "Missing required command: $command"
    exit 1
  fi
done

CI=true pnpm install --frozen-lockfile
pnpm spike:check
pnpm spike:storage
pnpm spike:security
pnpm spike:keychain
pnpm tauri:build
