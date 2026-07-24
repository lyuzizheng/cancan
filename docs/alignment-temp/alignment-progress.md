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
| Gmail data and cloud AI consent | partial | Validate the project-owned public Desktop OAuth client and Google verification path; decide exact Gmail attachment/message-body data sent to AI, separate transaction-notification body consent, provider retention, and Limited Use compatibility. The manually imported cloud OCR payload/config decision is accepted in `docs/specs/0004-parser-contract.md` for the mock-only provider-contract checkpoint; Gmail attachment/message-body consent remains unresolved. Direct local-client access, explicit user authorization, send-to-self collection through the user's mailbox, and separate development/test credentials are accepted | `docs/specs/0003-gmail-collector.md`, `docs/specs/0004-parser-contract.md`, `docs/specs/0007-first-run-onboarding.md` |

## P1: required before the affected implementation slice

| Area | Status | Decision still required | Canonical home after decision |
| --- | --- | --- | --- |
| First-run optionality | unresolved | Gmail may be skipped and local capture remains useful. Decide whether AI setup may be skipped and how any incomplete setup resumes | `docs/specs/0007-first-run-onboarding.md` |
| Build and open-source release details | partial | Decide minimum supported macOS version, project license, community governance/maintainer authority, signing/notarization identities, updater-key custody, stable/beta and security-update policy, release approval, immutable-release policy, and whether independently signed parser packs are ever allowed. Phase 1 macOS `arm64` plus `x86_64`, Windows Phase 2, GitHub Releases, semantic versions, signed artifacts, provenance/SBOM evidence, and no proprietary updater backend are accepted | `docs/specs/0018-app-updates-open-source-release.md` |
| Public project identity and website launch | unresolved | Choose the public domain, app/repository identity, support and security contacts, Cloudflare account/zone owner, privacy-policy owner, and final public copy; a static Cloudflare Pages site and GitHub-native community surfaces are accepted | `docs/specs/0018-app-updates-open-source-release.md` |
| Component foundation and Figma role | partial | The `Precision Vaultpunk` direction, version-1 accessible palette, typography roles, and motion rhythm are accepted. Decide the component-library major/version contract and whether Figma is authoritative, exploratory, or omitted | `docs/specs/0008-design-system.md` |
| Security observability and sensitive-data lifecycle | partial | Decide log redaction, opt-in crash-report details, and full-document extraction/index retention. Bounded validated raw source objects per external record and source-file tombstone deletion are accepted; MVP analytics/behavioral telemetry is excluded | future focused security spec after the decision |
| Backup operations and portability | unresolved | Manual/automatic schedule, failure UX, Command Center status, export formats, and restore-migration detail | `docs/specs/0009-backup-restore-versioning.md`, `docs/specs/0015-job-engine-error-model.md` |
| Local Inbox filesystem readiness | blocked | [Ordinary local temp-folder evidence](../../spikes/local-inbox-readiness/EVIDENCE.md) now proves stable handle-bound capture, deletion suppression, fresh rescan recovery, and fail-closed read paths. It does not prove app/process-restart lifecycle. The supported iCloud Drive path is absent or unreadable on the evidence machine, so actual cloud-placeholder/offline behavior and the settle interval remain unproven; keep the blocker until a supported iCloud run exists | `docs/specs/0017-evidence-documents-source-ux.md`, `docs/specs/0016-testing-fixtures-agent-gates.md` |

## P2: deliberately deferred, does not block the core MVP slices

| Area | Status | Decision still required | Canonical home after decision |
| --- | --- | --- | --- |
| Usage cadence and manual money | unresolved | Retention workflow, physical cash, and manual asset/liability scope | future focused product spec after the decision |
| AI Assistant operating scope | unresolved | Tool set, history, suggestion authority, and settings access after the core loop is stable | `docs/specs/0006-command-center-ui.md` or a future focused spec |
| Optional hosted AI service | partial | A paid CanCan-hosted relay is allowed later behind the existing provider-adapter/capability boundary, but its identity, payment, entitlement, minimal-state, retention, and privacy contracts are deferred. MVP remains direct BYO endpoint/model/API key | future focused service and release spec |
| Post-MVP extensions | blocked | Windows desktop is accepted for Phase 2 but still needs exact OS/architecture, installer/updater, signing, secret-store, filesystem, and validation contracts. A thin native iOS Share Extension/containing-app intake direction is accepted: a disposable local Xcode slice must choose the transport/encryption/offline/pairing/native-target boundary, while a later authorized mobile release slice owns signing, TestFlight, and App Store evidence. Mobile ledger/multi-device sync, API connectors, valuation providers, tax reporting, and parser sharing remain gated on the core loop | `docs/specs/0018-app-updates-open-source-release.md` plus future roadmap after MVP evidence exists |
