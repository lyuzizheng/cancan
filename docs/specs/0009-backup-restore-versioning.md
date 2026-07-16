# 0009. Backup, Restore, and Versioning Spec

## Goal

Define how CanCan preserves local-first data safely across app upgrades, backups, restores, and schema versions.

## Validated security boundary

The user-facing key model, Argon2id profiles, macOS Keychain scope, no-temporary-plaintext viewer boundary, source-file deletion contract, authenticated envelope, and restore-to-new-path switch are accepted. The reproducible evidence and residual architecture boundary are recorded in the [Vault security validation](../../spikes/vault-security-validation/EVIDENCE.md). Backup scheduling, export/migration behavior, public release security, and real macOS `x86_64` qualification remain owned by their later slices; they do not block architecture-neutral manual-import implementation with synthetic or redacted data.

## Simple user security model

MVP exposes only three security concepts:

```text
Vault password
Remember on this Mac
Recovery file
```

Do not expose key hierarchies, KDF parameters, per-file keys, key rotation, or a separate backup password in normal UI.

Rules:

- the vault password creates and unlocks the local vault;
- `Remember on this Mac` is an explicit opt-in backed by OS Keychain/secret storage;
- the recovery file is generated once, strongly recommended, and saved outside the vault;
- CanCan has no server-side password reset or recovery service;
- losing both password access and the recovery file makes the vault unrecoverable;
- MVP has no user-facing key-rotation workflow;
- system sleep/lock locks the vault immediately;
- fifteen minutes of inactivity locks the vault in MVP rather than adding another settings panel.

## Internal key model

Keep implementation robust but hidden:

```text
random vault master key
password -> Argon2id wrapping key -> wrapped master key
master key -> context-separated DB, file, identifier-digest, and backup subkeys
optional Keychain wrapper for Remember on this Mac
recovery file wrapper for the same master key
```

Changing the password re-wraps the master key rather than re-encrypting every record and file. Context-separated subkeys are implementation details and must not become user concepts.

Argon2id derives a wrapping key only when creating the password wrapper or unlocking through the vault password. It is not run once per document and does not encrypt document bytes. Normal unlock may use the opt-in Keychain wrapper, so `Remember on this Mac` avoids the password KDF on the daily path while preserving immediate lock on system sleep/lock.

The password-wrapper format stores a versioned KDF profile and its parameters. Phase 1 uses the same versioned profiles on macOS `arm64` and `x86_64`:

```text
rfc9106-low-memory-v1: Argon2id, m=64 MiB, t=3, p=4, 128-bit random salt, 256-bit wrapping key
owasp-minimum-v1:      Argon2id, m=19 MiB (19,456 KiB), t=2, p=1, 128-bit random salt, 256-bit wrapping key
```

The primary values follow [RFC 9106 section 4's second recommended option](https://www.ietf.org/rfc/rfc9106.html#section-4); the fallback floor follows the [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html#password-hashing-algorithms).

New vault creation benchmarks the primary RFC 9106 low-memory profile on the supported Mac and uses it when a password unlock completes within 750 ms. If it misses that UX budget, creation uses the OWASP minimum profile. Existing wrappers always retain their stored profile; opening a vault never silently changes the KDF contract. These profiles are not user settings.

The Phase 1 authenticated envelope is `CCENV001`, version 1. Its fixed header is 24 bytes and has this big-endian binary layout:

```text
offset  size  value
0       8     ASCII `CCENV001`
8       1     version: 1
9       1     purpose: file=1, password-wrapper=2, recovery-wrapper=3, backup=4
10      1     algorithm: XChaCha20-Poly1305=1
11      1     KDF profile: none=0, rfc9106-low-memory-v1=1, owasp-minimum-v1=2
12      2     salt length: unsigned 16-bit integer
14      2     nonce length: unsigned 16-bit integer, always 24
16      8     ciphertext length: unsigned 64-bit integer
24      n     salt, then 24-byte nonce, then ciphertext and authentication tag
```

`ciphertext length` includes the 16-byte Poly1305 tag, so it is the plaintext length plus 16. Algorithm 1 uses a 256-bit key and a fresh 192-bit nonce; the complete serialized header through the nonce is associated data.

Password wrappers carry exactly one 128-bit salt and one of the stored Argon2id profiles. Non-KDF file, recovery, and backup envelopes carry no salt. HKDF-SHA-256 derives purpose-separated file and backup keys from the master key with the versioned contexts `cancan:file:v1` and `cancan:backup:v1`. A reader rejects unknown versions, purposes, algorithms, KDF profiles, invalid lengths, wrong-purpose keys, wrong credentials, and any header or ciphertext tampering. Changing these bytes or contexts requires a new envelope version; existing version-1 data is never silently rewritten.

## Feasibility evidence

The [2026-07-13 disposable spike](../../spikes/desktop-feasibility/EVIDENCE.md) verified that the accepted user model is technically feasible on macOS arm64:

- Argon2id derived a wrapping key in 250-266 ms across two runs on the test machine;
- one random master key can be recovered through independent password and recovery wrappers;
- XChaCha20-Poly1305 file encryption rejects tampering and does not expose the synthetic plaintext;
- macOS Keychain can write, read, and delete the binary remember-on-device secret;
- SQLCipher can reject a wrong database key while supporting FTS5.

The later [Vault security validation](../../spikes/vault-security-validation/EVIDENCE.md) promoted the version-1 envelope and wrapper fixtures to the production compatibility contract and validated tamper, wrong-key, KDF-profile, Keychain, deletion, and logical restore crash behavior with independent review. A third-party security audit is not a release blocker for the current local MVP.

That feasibility evidence is `arm64`-only. Phase 1 support requires the production KDF benchmark, SQLCipher/file encryption, Keychain, in-memory viewer, deletion recovery, and backup/restore gates to pass independently on macOS `x86_64`; results from one architecture do not qualify the other.

## Source-file and statement-password policy

The encrypted file belongs to its `source_documents` registry row; MVP has no separate `vault_files` table and stores no second unlocked source copy.

Normal viewing decrypts and renders requested pages in memory inside Rust/Tauri. It does not create a plaintext temporary file. `Save a copy` is an explicit warned export to a user-selected path outside the Vault.

One optional statement-PDF password may be saved per Money Source in macOS Keychain. SQLite stores only its secret reference and status. Gmail and manual imports assigned to that Money Source try the same password. If it does not unlock a document, CanCan asks for a password and offers `Use once` or `Update saved password`; MVP stores no password history and no unlocked duplicate of the PDF.

## Backup target

MVP should support generic folder backup first. iCloud Drive is treated as a common folder target, not a hard dependency.

## Backup bundle

Recommended bundle:

```text
YYYY-MM-DD_HHMMSS.financevault
  manifest.json
  finance.sqlite
  files/
  audit.log
  checksums.json
```

The bundle should be encrypted before it leaves the local vault area.

File collection includes only `source_documents` whose current encrypted Vault file is available. A source-document tombstone, its metadata, bounded record evidence, audit, and ledger relationships remain in the database backup, but a deleted source file is not copied into a new backup. Existing immutable backups may still contain files deleted later.

Use the same vault/recovery model for backup access. MVP must not ask the user to remember a second backup password. A backup may carry independently salted/wrapped key metadata, but it must be recoverable through the accepted vault password or recovery-file flow without storing raw vault key material.

## Manifest requirements

```json
{
  "vault_version": 1,
  "app_version": "0.1.0",
  "schema_version": 1,
  "min_reader_app_version": "0.1.0",
  "created_at": "2026-07-08T00:00:00Z",
  "device_id": "local-device-id",
  "file_count": 0,
  "encrypted": true,
  "checksums": "checksums.json"
}
```

## Version compatibility

[0018 App Updates and Open-Source Release](0018-app-updates-open-source-release.md) owns app artifact delivery and update UX. This spec owns whether an updated or downgraded app may safely read, migrate, restore, or mutate a vault.

Newer app reading older vault:

```text
allowed if migration path exists
show migration preview/backup prompt
migrate safely
record migration audit entry
```

Older app reading newer vault:

```text
must not silently open/mutate
show upgrade-required message
explain current app version and required app/schema version
```

## Data model versioning

Every persisted shape with long-term meaning should carry enough version context:

- database schema version;
- parser version;
- prompt/template version;
- provider integration version;
- backup bundle version;
- app version that wrote the record when useful.

## Restore flow

```text
Choose backup bundle
-> verify manifest
-> verify checksums
-> verify encryption/password/key
-> check schema/app compatibility
-> restore to and validate a new inactive local Vault path
-> atomically switch the active Vault after confirmation
-> force re-authentication for secrets unless future secret backup is explicitly supported
```

## Secrets policy

Backups should not include secrets by default.

Excluded by default:

```text
Gmail OAuth refresh token
AI provider keys
statement PDF passwords
future read-only API tokens
vault key material
```

Restore writes and validates a new local Vault path before an atomic switch. It must not mutate the active Vault while integrity, compatibility, or password/recovery validation is incomplete.

After restore on a new device, CanCan opens the restored non-secret data and a resumable Setup Checklist. The checklist includes Gmail reconnect, AI provider key re-entry, statement-PDF password re-entry for each affected Money Source, `Remember on this Mac`, future API connector tokens, and backup-folder selection. Missing secrets block only the jobs or features that depend on them; the user may browse restored records and finish setup later.

## Acceptance criteria

- Backup has manifest, checksums, schema/app version markers.
- User-facing security is limited to vault password, optional Keychain remembering, and one recovery file.
- MVP has no server recovery, key-rotation UI, or separate backup password.
- Internal DB/file/identifier/backup keys are context-separated without becoming user settings.
- New app can migrate old vaults when supported.
- Old app refuses newer vaults with a clear upgrade message.
- Restore verifies integrity before replacing active data.
- Secrets are not restored silently.
- Argon2id parameters are stored as versioned wrapper profiles; new macOS vaults prefer RFC 9106's 64 MiB profile within the unlock budget and may fall back only to the OWASP minimum profile.
- `Remember on this Mac` uses Keychain to keep the common unlock path fast; Argon2id is not a per-document encryption step.
- Normal document viewing uses in-memory rendering without a plaintext temporary file; only explicit `Save a copy` exports plaintext.
- Statement passwords are optional one-per-Money-Source Keychain secrets, excluded from backups, with use-once/update behavior and no password history.
- Deleted source files are absent from future backups while their tombstones and relationships remain; older backup copies are not claimed to be erased.
- Restore validates a new Vault before switching and opens a resumable new-device Setup Checklist without hiding restored non-secret data.
- Phase 1 Vault/security compatibility is verified on both macOS `arm64` and `x86_64`; Windows uses a separate Phase 2 security/storage contract.
