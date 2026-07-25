#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
cd "$ROOT"

cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked

PREFLIGHT_OUTPUT="$(xcrun swift scripts/icloud-download-status.swift --self-test)"
printf '%s\n' "$PREFLIGHT_OUTPUT"
/usr/bin/grep -Fqx 'self-test=passed' <<<"$PREFLIGHT_OUTPUT"

scripts/run-local-evidence.sh
