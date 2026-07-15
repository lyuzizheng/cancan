# Vault security validation evidence

Status: pending GitHub CI evidence

This disposable slice evaluates a candidate production compatibility contract.
No real Vault data is accepted and no implementation blocker is removed until
the macOS CI gate passes and the result receives independent review.

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

## Required result

The workflow must publish the measured KDF timings and deterministic hashes in
its log while tests prove wrong-key/tamper rejection, no plaintext PDF file,
Keychain cleanup, deletion crash convergence, and the accepted `fsync`/rename
ordering at logical restore crash checkpoints. The simulation does not claim to
reproduce hardware, filesystem, or storage-device failures that violate those
durability primitives.
