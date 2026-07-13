#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SETUP="$ROOT/scripts/setup-dev.sh"
VERSIONS="$ROOT/scripts/dev-toolchain.env"

fail() {
  echo "setup-dev test failed: $*" >&2
  exit 1
}

assert_equal() {
  [[ "$1" == "$2" ]] || fail "expected '$2', got '$1'"
}

assert_contains() {
  [[ "$1" == *"$2"* ]] || fail "output did not contain: $2"
}

[[ -x "$SETUP" ]] || fail "$SETUP is missing or not executable"
bash -n "$SETUP"
grep -Fq -- '--no-modify-path' "$SETUP" || fail "rustup may modify unrelated shell profiles"
if grep -Fq '"$rustup" default' "$SETUP"; then
  fail "setup may change the user's global Rust default"
fi

# shellcheck source=dev-toolchain.env
source "$VERSIONS"

assert_equal "$(tr -d '[:space:]' < "$ROOT/.node-version")" "$NODE_VERSION"
grep -Fq "channel = \"$RUST_VERSION\"" "$ROOT/rust-toolchain.toml" || fail "Rust pin drifted"
grep -Fq "\"packageManager\": \"pnpm@$PNPM_VERSION\"" \
  "$ROOT/package.json" || fail "root pnpm pin drifted"
grep -Fq "\"packageManager\": \"pnpm@$PNPM_VERSION\"" \
  "$ROOT/spikes/desktop-feasibility/package.json" || fail "pnpm pin drifted"
grep -Fq "\"@tauri-apps/cli\": \"$TAURI_CLI_VERSION\"" \
  "$ROOT/apps/desktop/package.json" || fail "application Tauri CLI pin drifted"
grep -Fq "\"@tauri-apps/cli\": \"$TAURI_CLI_VERSION\"" \
  "$ROOT/spikes/desktop-feasibility/package.json" || fail "Tauri CLI pin drifted"
grep -Fq 'pnpm --ignore-workspace install --frozen-lockfile' \
  "$ROOT/spikes/desktop-feasibility/scripts/verify.sh" || fail "spike lockfile isolation drifted"
grep -Fq 'cargo clippy --locked' \
  "$ROOT/spikes/desktop-feasibility/package.json" || fail "spike Cargo lock drifted"

# shellcheck source=setup-dev.sh
source "$SETUP"

verification_function="$(declare -f run_verification)"
assert_contains "$verification_function" 'pnpm verify'
assert_contains "$verification_function" 'pnpm spike:verify'
production_gate_line="$(printf '%s\n' "$verification_function" | grep -nF 'pnpm verify' | cut -d: -f1)"
spike_gate_line="$(printf '%s\n' "$verification_function" | grep -nF 'pnpm spike:verify' | cut -d: -f1)"
[[ "$production_gate_line" -lt "$spike_gate_line" ]] || fail "verification gate order drifted"

assert_equal "$(node_archive_arch arm64)" "arm64"
assert_equal "$(node_archive_arch x86_64)" "x64"
if node_archive_arch riscv64 >/dev/null 2>&1; then
  fail "unsupported architecture was accepted"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

payload="$tmp/payload"
printf 'cancan setup checksum\n' > "$payload"
checksum="$(shasum -a 256 "$payload" | awk '{print $1}')"
verify_sha256 "$payload" "$checksum"
if verify_sha256 "$payload" "0000000000000000000000000000000000000000000000000000000000000000" >/dev/null 2>&1; then
  fail "invalid checksum was accepted"
fi

foreign_tool="$tmp/foreign-tool"
printf 'keep me\n' > "$foreign_tool"
if (link_cancan_tool "$tmp/replacement" "$foreign_tool") >"$tmp/foreign-file.out" 2>&1; then
  fail "foreign regular tool was accepted for replacement"
fi
assert_equal "$(<"$foreign_tool")" "keep me"

foreign_dir="$tmp/foreign-dir"
mkdir "$foreign_dir"
if (link_cancan_tool "$tmp/replacement" "$foreign_dir") >"$tmp/foreign-dir.out" 2>&1; then
  fail "foreign tool directory was accepted for replacement"
fi
[[ -d "$foreign_dir" ]] || fail "foreign tool directory was not preserved"

foreign_link="$tmp/foreign-link"
ln -s "$tmp/owned-elsewhere" "$foreign_link"
if (link_cancan_tool "$tmp/replacement" "$foreign_link") >"$tmp/foreign-link.out" 2>&1; then
  fail "foreign tool symlink was accepted for replacement"
fi
assert_equal "$(readlink "$foreign_link")" "$tmp/owned-elsewhere"

managed_home="$tmp/managed-home"
mkdir -p "$managed_home/.local/bin"
ln -s '../share/cancan/node-v-old/bin/node' "$managed_home/.local/bin/node"
managed_target="$managed_home/.local/share/cancan/node-v-new/bin/node"
HOME="$managed_home" link_cancan_tool "$managed_target" "$managed_home/.local/bin/node"
assert_equal "$(readlink "$managed_home/.local/bin/node")" "$managed_target"

fake_fnm="$tmp/fnm"
printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$*" >> "$CANCAN_TEST_FNM_LOG"\n' > "$fake_fnm"
chmod +x "$fake_fnm"
CANCAN_TEST_FNM_LOG="$tmp/fnm.log" ensure_fnm_node "$fake_fnm"
assert_equal "$(sed -n '1p' "$tmp/fnm.log")" "install $NODE_VERSION"
assert_equal "$(sed -n '2p' "$tmp/fnm.log")" \
  "exec --using $NODE_VERSION npm install --global corepack@$COREPACK_VERSION"
assert_equal "$(sed -n '3p' "$tmp/fnm.log")" \
  "exec --using $NODE_VERSION corepack install --global pnpm@$PNPM_VERSION"
assert_equal "$(sed -n '4p' "$tmp/fnm.log")" \
  "exec --using $NODE_VERSION corepack enable pnpm"

HOME="$tmp/home"
CANCAN_SETUP_PROFILE="$HOME/.zprofile"
DRY_RUN=0
mkdir -p "$HOME"
ensure_shell_path >/dev/null
ensure_shell_path >/dev/null
profile_line='export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"'
assert_equal "$(grep -Fxc "$profile_line" "$CANCAN_SETUP_PROFILE")" "1"

dry_arm="$({
  CANCAN_SETUP_OS=Darwin \
  CANCAN_SETUP_ARCH=arm64 \
  CANCAN_SETUP_XCODE_READY=0 \
  CANCAN_SETUP_PROFILE="$tmp/dry-arm-profile" \
    bash "$SETUP" --dry-run --skip-verify
} 2>&1)"
assert_contains "$dry_arm" "xcode-select --install"
assert_contains "$dry_arm" "node-v$NODE_VERSION-darwin-arm64.tar.gz"
assert_contains "$dry_arm" "corepack@$COREPACK_VERSION"
assert_contains "$dry_arm" "pnpm@$PNPM_VERSION"
assert_contains "$dry_arm" "Rust $RUST_VERSION"
assert_contains "$dry_arm" "Tauri CLI $TAURI_CLI_VERSION (project-local)"

dry_x64="$({
  CANCAN_SETUP_OS=Darwin \
  CANCAN_SETUP_ARCH=x86_64 \
  CANCAN_SETUP_XCODE_READY=1 \
  CANCAN_SETUP_PROFILE="$tmp/dry-x64-profile" \
    bash "$SETUP" --dry-run --skip-verify
} 2>&1)"
assert_contains "$dry_x64" "node-v$NODE_VERSION-darwin-x64.tar.gz"

if CANCAN_SETUP_OS=Linux bash "$SETUP" --dry-run --skip-verify >"$tmp/linux.out" 2>&1; then
  fail "unsupported operating system was accepted"
fi
grep -Fq "supports macOS only" "$tmp/linux.out" || fail "unsupported OS error was unclear"

echo "setup-dev tests passed"
