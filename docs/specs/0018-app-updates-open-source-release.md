# 0018. App Updates, Forward Compatibility, and Open-Source Release Spec

## Goal

Define how CanCan ships frequent app and provider-parser updates through a classic fully open-source GitHub release flow without breaking local vaults or silently changing committed financial facts.

## Implementation blocker

MVP operating systems, project license, desktop runtime acceptance, signing/notarization identities, updater key management, update channels, release approval policy, and whether parser packages may ever update independently remain unresolved in the [active alignment register](../alignment-temp/alignment-progress.md).

Do not implement an unsigned updater, remote prompt/config download, or platform-specific release pipeline by inference.

## Stable decisions

- CanCan is a fully open-source application.
- The public Git repository, version tags, source history, build workflow, and GitHub Releases are the canonical release record.
- App versions use semantic versioning.
- A release is traceable from artifact to immutable source tag and CI run.
- MVP does not require a proprietary CanCan update backend.
- App/parser updates never silently rewrite committed ledger events.

## GitHub release pipeline

Target flow:

```text
version change and changelog
-> reviewed commit on main
-> immutable semantic-version tag
-> GitHub Actions deterministic tests and harness gates
-> platform builds
-> platform signing/notarization where required
-> checksums, signatures, update metadata, and dependency/license report
-> draft GitHub Release
-> release approval
-> publish artifacts and release notes
```

Release artifacts should include, where applicable:

```text
source tag/archive
human-readable changelog
platform installer/package
SHA-256 checksums
cryptographic signatures and signed update metadata
dependency/license or SBOM-style report
migration/compatibility notes
```

Publish binaries from CI, not an unrecorded developer-machine build. A platform artifact must not be advertised as supported when its required signing or compatibility gate did not pass.

## Incremental app update behavior

The app may check release metadata backed by GitHub Releases only when the user enables update checks or invokes a manual check. It must verify signed update metadata and artifact signatures before installation.

The normal UX should show:

```text
current and available version
release notes
download size when known
whether restart is required
compatibility or migration warning
install now / remind later
```

Do not force an update silently. Security-critical update policy and stable/beta channels remain unresolved.

If ADR 0001 is accepted, the implementation may use the Tauri signed updater with GitHub-hosted artifacts. Until then, this spec does not make Tauri an accepted release dependency.

## Provider-parser delivery

Provider parser packages, prompts, schemas, validators, and qualification metadata change frequently and carry explicit versions.

Safe MVP boundary:

```text
ship parser updates inside signed app releases
do not download unsigned prompts, parser logic, or qualification config at runtime
preserve the exact parser/prompt/schema/validator versions used by every parse run
new parser versions start unqualified and follow fixture + shadow gates
installing an app update does not automatically reparse or mutate committed facts
```

A later independently signed parser-pack channel requires a separate accepted design for package signatures, compatibility, rollback, revocation, and update authority.

## Forward compatibility

[0009 Backup, Restore, and Versioning](0009-backup-restore-versioning.md) owns vault/schema/backup compatibility. App updates must obey its manifest, minimum-reader, pre-migration backup, and no-unsafe-downgrade rules.

Additional parser compatibility rules:

- old parse runs remain readable with their recorded package versions;
- new parsers produce new versions rather than overwriting previous outputs;
- optional reparse creates proposals/review work and never silently changes committed ledger facts;
- removed provider support must retain historical rendering and evidence access;
- unsupported newer data causes a clear upgrade-required/read-only failure, not best-effort mutation.

## Rollback

Keep prior public release artifacts available unless a security incident requires revocation.

Application rollback does not imply data rollback. An older app must refuse write access when the current vault/schema/parser metadata requires a newer reader. Data rollback uses verified backup/restore behavior owned by [0009](0009-backup-restore-versioning.md).

## Acceptance criteria

- Every published app artifact traces to a public immutable source tag and GitHub Actions run.
- Releases include checksums, signatures where required, notes, and compatibility information.
- Update artifacts are signature-verified and are not silently forced.
- Parser updates are versioned, qualified independently, and cannot silently rewrite committed facts.
- MVP does not download unsigned parser/prompt/config updates.
- Older incompatible apps refuse mutation and explain the required version.
- Tauri-specific updater implementation remains conditional on ADR 0001 acceptance.
