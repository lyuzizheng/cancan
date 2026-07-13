# Desktop Feasibility Evidence

Date: 2026-07-13

This evidence applies only to the disposable `desktop-feasibility` slice. It accepts the runtime architecture, not a production cryptographic format.

## Environment

```text
macOS 26.5.2 arm64
Apple Command Line Tools / clang 21.0.0
Rust 1.97.0 stable
Node.js 24.14.0
pnpm 11.7.0
Tauri CLI 2.11.4
Tauri crate 2.11.5
rusqlite 0.40.1
bundled SQLCipher 4.14.0 community
```

The local machine initially lacked Rust. It was installed with the minimal stable rustup profile, then `rustfmt` and `clippy` were added. Tauri's official prerequisite guide requires Rust and allows Command Line Tools for macOS desktop-only development: <https://v2.tauri.app/start/prerequisites/>.

## Reproducible gate

```text
pnpm spike:verify
```

Observed results:

```text
cargo fmt --check: passed
cargo clippy --all-targets -- -D warnings: passed
cargo test: 2 passed
SQLCipher: 4.14.0 community; wrong key rejected; plaintext sentinel absent
FTS5: virtual table creation and MATCH query passed in the SQLCipher build
Argon2id spike parameters: 19,456 KiB, 2 iterations, 1 lane; 250-266 ms across two runs
password wrapper: passed; wrong password rejected
recovery wrapper: passed
XChaCha20-Poly1305 file round trip: passed; tampering rejected; plaintext absent
macOS Keychain: binary secret write/read/delete passed; cleanup confirmed
Tauri debug desktop build without bundle: passed
```

The SQLCipher build needs `LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_FTS5`; the spike pins that build flag in `src-tauri/.cargo/config.toml`.

## Accepted conclusion

Tauri 2 + a Rust privileged boundary + bundled SQLCipher/SQLite is feasible on the current macOS arm64 development target. SQLCipher and FTS5 can coexist, and the accepted three-concept security UX can be implemented using standard password KDF, authenticated encryption, recovery wrapping, and OS secret-store primitives. The production React/TypeScript shell remains the first gate of `app-foundation`; this spike used a static frontend so it could isolate the privileged desktop/storage path.

## Still blocked

This spike does not choose or promise:

- production Argon2id parameters or compatibility format;
- the production encrypted-file/recovery envelope;
- temporary plaintext opening and crash-cleanup behavior;
- macOS protected-data versus legacy Keychain mode;
- Windows/Linux secret-store behavior;
- atomic backup restore or destructive-job semantics;
- release targets, code-signing identities, notarization, or updater keys.

Those remain owned by the canonical security/release specs and active alignment register.
