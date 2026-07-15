# Active Alignment Register

This file contains only decisions that are still partial, unresolved, or blocked. Stable decisions belong in canonical specs or status-bearing ADRs and must not be copied here.

Status values:

```text
partial       direction exists but implementation-critical detail is missing
unresolved    user discussion is required
blocked       external evidence, feasibility work, or implementation is required first
```

## P0: blocks safe implementation

| Area | Status | Decision still required | Canonical home after decision |
| --- | --- | --- | --- |
| Vault/file/backup security validation | blocked | Validate the accepted versioned Argon2id profiles and unlock budget, authenticated-file and recovery envelope formats, macOS Keychain behavior, no-temporary-plaintext viewer, deletion crash recovery, backup recovery, and atomic restore with compatibility fixtures and independent code review | `docs/specs/0009-backup-restore-versioning.md` and an ADR if architecture changes |
| Gmail data and cloud AI consent | partial | Validate the project-owned public Desktop OAuth client and Google verification path; decide exact data sent to AI, consent granularity, provider retention, and Limited Use compatibility. Direct local-client access, explicit user authorization, and separate development/test credentials are accepted | `docs/specs/0003-gmail-collector.md`, `docs/specs/0007-first-run-onboarding.md` |
| Restore and destructive job behavior | partial | Prove the accepted restore-to-new-path/atomic-switch and source-file-tombstone outcomes with exact cancellation boundaries, crash state transitions, and idempotency keys | `docs/specs/0015-job-engine-error-model.md`, `docs/specs/0009-backup-restore-versioning.md` |

## P1: required before the affected implementation slice

| Area | Status | Decision still required | Canonical home after decision |
| --- | --- | --- | --- |
| First-run optionality | unresolved | Whether AI and Gmail setup can be skipped and how incomplete setup resumes | `docs/specs/0007-first-run-onboarding.md` |
| Build and open-source release details | partial | Decide minimum supported macOS version, project license, community governance/maintainer authority, signing/notarization identities, updater-key custody, stable/beta and security-update policy, release approval, immutable-release policy, and whether independently signed parser packs are ever allowed. Phase 1 macOS `arm64` plus `x86_64`, Windows Phase 2, GitHub Releases, semantic versions, signed artifacts, provenance/SBOM evidence, and no proprietary updater backend are accepted | `docs/specs/0018-app-updates-open-source-release.md` |
| Public project identity and website launch | unresolved | Choose the public domain, app/repository identity, support and security contacts, Cloudflare account/zone owner, privacy-policy owner, and final public copy; a static Cloudflare Pages site and GitHub-native community surfaces are accepted | `docs/specs/0018-app-updates-open-source-release.md` |
| Visual tokens and component version | unresolved | Exact accessible token values, HeroUI major/version contract, Figma role | `docs/specs/0008-design-system.md`, `docs/specs/0011-visual-design-tokens.md` |
| Security observability and sensitive-data lifecycle | partial | Decide log redaction, opt-in crash-report details, and full-document extraction/index retention. Bounded validated raw source objects per external record and source-file tombstone deletion are accepted; MVP analytics/behavioral telemetry is excluded | future focused security spec after the decision |
| Backup operations and portability | unresolved | Manual/automatic schedule, failure UX, Command Center status, export formats, and restore-migration detail | `docs/specs/0009-backup-restore-versioning.md`, `docs/specs/0015-job-engine-error-model.md` |

## P2: deliberately deferred, does not block the core MVP slices

| Area | Status | Decision still required | Canonical home after decision |
| --- | --- | --- | --- |
| Usage cadence and manual money | unresolved | Retention workflow, physical cash, and manual asset/liability scope | future focused product spec after the decision |
| AI Assistant operating scope | unresolved | Tool set, history, suggestion authority, and settings access after the core loop is stable | `docs/specs/0006-command-center-ui.md` or a future focused spec |
| Post-MVP extensions | blocked | Windows desktop is accepted for Phase 2 but still needs exact OS/architecture, installer/updater, signing, secret-store, filesystem, and validation contracts. Mobile, API connectors, valuation providers, tax reporting, parser sharing, and multi-device sync also remain gated on the core loop | `docs/specs/0018-app-updates-open-source-release.md` plus future roadmap after MVP evidence exists |
