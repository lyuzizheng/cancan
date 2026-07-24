#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
cd "$ROOT"

cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
scripts/check-icloud-path.sh
