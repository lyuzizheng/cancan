#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=dev-toolchain.env
source "$ROOT/scripts/dev-toolchain.env"

DRY_RUN=0
SKIP_VERIFY=0
TEMP_DIR=""

log() {
  echo "[cancan setup] $*"
}

die() {
  echo "[cancan setup] error: $*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Usage: ./scripts/setup-dev.sh [--dry-run] [--skip-verify]

Installs the pinned CanCan macOS development toolchain without sudo or Homebrew.
The first run may open Apple's Command Line Tools installer; rerun this command
after that installer completes.
EOF
}

node_archive_arch() {
  case "$1" in
    arm64) echo "arm64" ;;
    x86_64) echo "x64" ;;
    *)
      echo "unsupported macOS architecture: $1" >&2
      return 1
      ;;
  esac
}

verify_sha256() {
  local file="$1"
  local expected="$2"
  local actual
  actual="$(shasum -a 256 "$file" | awk '{print $1}')"
  [[ "$actual" == "$expected" ]]
}

ensure_replaceable_tool_path() {
  local path="$1"
  local target remainder
  if [[ ! -e "$path" && ! -L "$path" ]]; then
    return
  fi

  if [[ -L "$path" ]]; then
    target="$(readlink "$path")"
    case "$target" in
      "$HOME/.local/share/cancan/"*) remainder="${target#"$HOME/.local/share/cancan/"}" ;;
      ../share/cancan/*) remainder="${target#../share/cancan/}" ;;
      *) remainder="" ;;
    esac
    if [[ -n "$remainder" && "$remainder" != *"/../"* && "$remainder" != ../* && "$remainder" != */.. ]]; then
      return
    fi
  fi

  die "refusing to replace existing tool at $path; move it or remove it explicitly, then rerun setup"
}

link_cancan_tool() {
  local target="$1"
  local path="$2"
  ensure_replaceable_tool_path "$path"
  ln -sfn "$target" "$path"
}

shell_profile() {
  if [[ -n "${CANCAN_SETUP_PROFILE:-}" ]]; then
    echo "$CANCAN_SETUP_PROFILE"
  elif [[ "${SHELL:-}" == */bash ]]; then
    echo "$HOME/.bash_profile"
  else
    echo "$HOME/.zprofile"
  fi
}

ensure_shell_path() {
  local profile line
  profile="$(shell_profile)"
  line='export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"'

  if [[ "$DRY_RUN" == "1" ]]; then
    log "Would ensure toolchain PATH in $profile"
    return
  fi

  mkdir -p "$(dirname "$profile")"
  touch "$profile"
  if ! grep -Fqx "$line" "$profile"; then
    printf '\n# CanCan development toolchain\n%s\n' "$line" >> "$profile"
  fi
}

cleanup() {
  if [[ -n "$TEMP_DIR" && -d "$TEMP_DIR" ]]; then
    rm -rf "$TEMP_DIR"
  fi
}

require_macos() {
  local os="${CANCAN_SETUP_OS:-$(uname -s)}"
  [[ "$os" == "Darwin" ]] || die "setup currently supports macOS only (found $os)"
}

require_command_line_tools() {
  local ready="${CANCAN_SETUP_XCODE_READY:-}"
  if [[ "$ready" == "1" ]] || { [[ -z "$ready" ]] && xcode-select -p >/dev/null 2>&1; }; then
    log "Apple Command Line Tools ready"
    return
  fi

  if [[ "$DRY_RUN" == "1" ]]; then
    log "Would run: xcode-select --install"
    return
  fi

  xcode-select --install >/dev/null 2>&1 || true
  cat >&2 <<'EOF'
[cancan setup] Apple opened the Command Line Tools installer.
[cancan setup] Finish that installation, then rerun ./scripts/setup-dev.sh.
EOF
  exit 2
}

install_node() {
  local machine_arch node_arch archive extracted base_url node_home local_bin expected
  machine_arch="${CANCAN_SETUP_ARCH:-$(uname -m)}"
  node_arch="$(node_archive_arch "$machine_arch")" || die "unsupported architecture"
  archive="node-v$NODE_VERSION-darwin-$node_arch.tar.gz"
  extracted="${archive%.tar.gz}"
  base_url="https://nodejs.org/dist/v$NODE_VERSION"
  node_home="$HOME/.local/share/cancan/node-v$NODE_VERSION"
  local_bin="$HOME/.local/bin"

  if [[ "$DRY_RUN" == "1" ]]; then
    log "Would install Node.js $NODE_VERSION from $base_url/$archive"
    return
  fi

  mkdir -p "$HOME/.local/share/cancan" "$local_bin"
  if [[ ! -x "$node_home/bin/node" ]] || [[ "$($node_home/bin/node --version)" != "v$NODE_VERSION" ]]; then
    TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cancan-setup.XXXXXX")"
    curl --proto '=https' --tlsv1.2 -fsSLo "$TEMP_DIR/$archive" "$base_url/$archive"
    curl --proto '=https' --tlsv1.2 -fsSLo "$TEMP_DIR/SHASUMS256.txt" "$base_url/SHASUMS256.txt"
    expected="$(awk -v name="$archive" '$2 == name || $2 == "*" name {print $1; exit}' "$TEMP_DIR/SHASUMS256.txt")"
    [[ -n "$expected" ]] || die "Node.js checksum entry not found for $archive"
    verify_sha256 "$TEMP_DIR/$archive" "$expected" || die "Node.js SHA-256 verification failed"
    tar -xzf "$TEMP_DIR/$archive" -C "$TEMP_DIR"
    [[ "$node_home" == "$HOME/.local/share/cancan/node-v"* ]] || die "unsafe Node.js install path"
    rm -rf "$node_home"
    mv "$TEMP_DIR/$extracted" "$node_home"
    cleanup
    TEMP_DIR=""
  fi

  link_cancan_tool "$node_home/bin/node" "$local_bin/node"
  link_cancan_tool "$node_home/bin/npm" "$local_bin/npm"
  link_cancan_tool "$node_home/bin/npx" "$local_bin/npx"
}

find_fnm() {
  local candidate
  if command -v fnm >/dev/null 2>&1; then
    command -v fnm
    return
  fi

  for candidate in "$HOME/.local/bin/fnm" /opt/homebrew/bin/fnm /usr/local/bin/fnm; do
    if [[ -x "$candidate" ]]; then
      echo "$candidate"
      return
    fi
  done
  return 1
}

ensure_fnm_node() {
  local fnm_bin="${1:-}"
  if [[ -z "$fnm_bin" ]] && ! fnm_bin="$(find_fnm)"; then
    return
  fi

  if [[ "$DRY_RUN" == "1" ]]; then
    log "Would make Node.js $NODE_VERSION, Corepack $COREPACK_VERSION, and pnpm $PNPM_VERSION available to existing fnm"
    return
  fi

  log "Existing fnm detected; preparing its project toolchain"
  "$fnm_bin" install "$NODE_VERSION"
  "$fnm_bin" exec --using "$NODE_VERSION" npm install --global "corepack@$COREPACK_VERSION"
  "$fnm_bin" exec --using "$NODE_VERSION" corepack install --global "pnpm@$PNPM_VERSION"
  "$fnm_bin" exec --using "$NODE_VERSION" corepack enable pnpm
}

install_corepack_and_pnpm() {
  local node_home="$HOME/.local/share/cancan/node-v$NODE_VERSION"
  local local_bin="$HOME/.local/bin"
  local stamp="$node_home/.cancan-corepack-$COREPACK_VERSION-pnpm-$PNPM_VERSION"

  if [[ "$DRY_RUN" == "1" ]]; then
    log "Would install corepack@$COREPACK_VERSION and activate pnpm@$PNPM_VERSION"
    return
  fi

  ensure_replaceable_tool_path "$local_bin/corepack"
  ensure_replaceable_tool_path "$local_bin/pnpm"
  ensure_replaceable_tool_path "$local_bin/pnpx"

  if [[ -f "$stamp" && -x "$local_bin/corepack" && -x "$local_bin/pnpm" ]] && \
    [[ "$($local_bin/corepack --version)" == "$COREPACK_VERSION" ]] && \
    [[ "$($local_bin/pnpm --version)" == "$PNPM_VERSION" ]]; then
    log "Corepack $COREPACK_VERSION and pnpm $PNPM_VERSION already ready"
    return
  fi

  "$node_home/bin/npm" install --global --prefix "$node_home" "corepack@$COREPACK_VERSION"
  link_cancan_tool "$node_home/bin/corepack" "$local_bin/corepack"
  "$local_bin/corepack" install --global "pnpm@$PNPM_VERSION"
  "$local_bin/corepack" enable pnpm --install-directory "$local_bin"
  touch "$stamp"
}

install_rust() {
  local rustup="$HOME/.cargo/bin/rustup"

  if [[ "$DRY_RUN" == "1" ]]; then
    log "Would install Rust $RUST_VERSION with clippy and rustfmt via rustup"
    return
  fi

  if [[ ! -x "$rustup" ]]; then
    TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cancan-rustup.XXXXXX")"
    curl --proto '=https' --tlsv1.2 -fsSLo "$TEMP_DIR/rustup-init.sh" https://sh.rustup.rs
    sh "$TEMP_DIR/rustup-init.sh" -y --profile minimal --default-toolchain none --no-modify-path
    cleanup
    TEMP_DIR=""
  fi

  "$rustup" toolchain install "$RUST_VERSION" \
    --profile minimal \
    --component clippy \
    --component rustfmt
}

bootstrap_dependencies() {
  local package_root
  if [[ -f "$ROOT/package.json" && -f "$ROOT/pnpm-lock.yaml" ]]; then
    package_root="$ROOT"
  else
    package_root="$ROOT/spikes/desktop-feasibility"
  fi

  if [[ "$DRY_RUN" == "1" ]]; then
    log "Would run pnpm install --frozen-lockfile in $package_root"
    return
  fi

  (cd "$package_root" && pnpm install --frozen-lockfile)
}

check_versions() {
  local actual
  if [[ "$DRY_RUN" == "1" ]]; then
    log "Pinned toolchain: Node $NODE_VERSION, Corepack $COREPACK_VERSION, pnpm $PNPM_VERSION, Rust $RUST_VERSION"
    log "Tauri CLI $TAURI_CLI_VERSION (project-local)"
    return
  fi

  [[ "$(node --version)" == "v$NODE_VERSION" ]] || die "Node.js version mismatch"
  [[ "$(corepack --version)" == "$COREPACK_VERSION" ]] || die "Corepack version mismatch"
  [[ "$(pnpm --version)" == "$PNPM_VERSION" ]] || die "pnpm version mismatch"
  rustc --version | grep -Fq "rustc $RUST_VERSION " || die "Rust version mismatch"
  cargo clippy --version >/dev/null
  cargo fmt --version >/dev/null
  actual="$(cd "$ROOT/spikes/desktop-feasibility" && pnpm exec tauri --version)"
  [[ "$actual" == *"$TAURI_CLI_VERSION"* ]] || die "Tauri CLI version mismatch: $actual"
  log "Verified Node $(node --version), Corepack $(corepack --version), pnpm $(pnpm --version), $(rustc --version)"
  log "Verified $actual (project-local)"
}

run_verification() {
  if [[ "$SKIP_VERIFY" != "0" ]]; then
    return 0
  fi
  if [[ "$DRY_RUN" == "1" ]]; then
    log "Would run agent preflight and the desktop feasibility gate"
    return
  fi
  "$ROOT/.agents/scripts/agent-preflight.sh"
  (cd "$ROOT/spikes/desktop-feasibility" && pnpm spike:verify)
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run) DRY_RUN=1 ;;
      --skip-verify) SKIP_VERIFY=1 ;;
      -h|--help) usage; return ;;
      *) die "unknown argument: $1" ;;
    esac
    shift
  done

  trap cleanup EXIT
  require_macos
  require_command_line_tools
  ensure_shell_path
  export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
  install_node
  ensure_fnm_node
  install_corepack_and_pnpm
  install_rust
  bootstrap_dependencies
  check_versions
  run_verification
  log "Development environment ready. Open a new shell or source $(shell_profile)."
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
