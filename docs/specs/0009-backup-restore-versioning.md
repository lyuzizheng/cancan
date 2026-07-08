# 0009. Backup, Restore, and Versioning Spec

## Goal

Define how CanCan preserves local-first data safely across app upgrades, backups, restores, and schema versions.

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
- New app can migrate old vaults when supported.
- Old app refuses newer vaults with a clear upgrade message.
- Restore verifies integrity before replacing active data.
- Secrets are not restored silently.
