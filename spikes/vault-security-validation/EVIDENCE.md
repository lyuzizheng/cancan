# Vault security validation evidence

Status: accepted on 2026-07-15

This disposable slice validated the production compatibility candidate without
accepting real Vault data. [PR #7](https://github.com/lyuzizheng/cancan/pull/7)
passed the complete macOS gate and independent security/code and semantic
review before merge.

## Candidate under test

- Argon2id profiles exactly match `rfc9106-low-memory-v1` and
  `owasp-minimum-v1` from the backup/restore spec.
- New Vault creation selects the RFC 9106 profile when its measured password
  derivation is within 750 ms and otherwise selects the OWASP floor.
- `CCENV001` is a big-endian binary envelope with explicit version, purpose,
  algorithm, KDF profile, salt, nonce, and ciphertext lengths.
- XChaCha20-Poly1305 authenticates the complete serialized header as associated
  data. HKDF-SHA-256 derives purpose-specific file and backup keys.
- Restore syncs and validates an inactive Vault before atomically replacing a
  small active-Vault locator. It never replaces the active directory in place.
- Source-file deletion commits the tombstone and audit decision before removing
  the encrypted blob; recovery converges any remaining blob deletion.

## Accepted result

The merged CI job passed formatting, Clippy, all Rust tests, and the combined
evidence command:

- [Vault security validation gate](https://github.com/lyuzizheng/cancan/actions/runs/29407345079/job/87325901950?pr=7)
- the measured primary/fallback KDF timings were published by the job and the
  profile selected according to the 750 ms rule;
- the deterministic file-envelope SHA-256 is
  `3b46ed1d62b420d3dc6e9ba62ddfc52b15c5d921b923eed49e279a2f0da6b7da`;
- the RFC password, OWASP password, and recovery wrapper fixture hashes are
  `d77cc533bf44270c08eeaaa64b05238b990d3156def3fd9cdd65a2dcccfd70e7`,
  `4fb57b26f766c5b0882ea54ff87075f722f55c2faf608407974a0f7f3f480f2d`,
  and `7a2edcc69e464bbf25928609659f8ada3ab0db7f96a15b5cc67dce85c2bcc4c4`;
- Keychain write/update/read/delete, in-memory Core Graphics rendering, four
  deletion crash cases, and six restore crash cases passed.

## Residual boundary

This evidence is macOS `arm64` only, uses synthetic data, and simulates logical
crash points assuming the tested `fsync` and atomic-rename primitives. A real
macOS `x86_64` build/runtime/security pass remains a `backup-release` gate.
