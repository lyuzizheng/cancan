# 0009. Backup, Restore, and Versioning Spec

## Goal

Define how CanCan preserves local-first data safely across app upgrades, backups, restores, and schema versions.

## Implementation blocker

The user-facing key model is accepted. Exact Argon2id parameters, file-encryption format, platform Keychain behavior, recovery-wrapper format, temporary-plaintext crash cleanup, atomic restore, destructive jobs, and portability require implementation/security validation in the [active alignment register](../alignment-temp/alignment-progress.md).

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
- system sleep/lock locks the vault, with a fixed conservative inactivity lock in MVP rather than another settings panel.

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

Exact KDF cost, salt, nonce, authenticated-encryption, key-version, and recovery-file formats must be fixed by the desktop/storage security spike and covered by compatibility fixtures before real data is accepted.

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
-> restore to new local vault path or replace existing after confirmation
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

After restore:

- Gmail reconnect required;
- AI provider key re-entry required unless a future explicit secret-backup design exists;
- statement PDF password re-entry required unless a future explicit secret-backup design exists;
- API connector token re-entry required.

## Acceptance criteria

- Backup has manifest, checksums, schema/app version markers.
- User-facing security is limited to vault password, optional Keychain remembering, and one recovery file.
- MVP has no server recovery, key-rotation UI, or separate backup password.
- Internal DB/file/identifier/backup keys are context-separated without becoming user settings.
- New app can migrate old vaults when supported.
- Old app refuses newer vaults with a clear upgrade message.
- Restore verifies integrity before replacing active data.
- Secrets are not restored silently.
