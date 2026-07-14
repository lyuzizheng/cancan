#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
cd "$ROOT"

CI=true pnpm install --frozen-lockfile
pnpm typecheck
pnpm test
pnpm spike:check
pnpm build:tauri
pnpm evidence
