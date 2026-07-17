# Current State

Last updated: 2026-07-17

## Product phase

CanCan has crossed from documentation/design alignment into production implementation. The app foundation and synthetic core flow are complete; production Vault storage is in progress, while live-document normalization and user-facing financial behavior have not started.

The current product target is a Gmail-first local financial evidence vault and polished personal finance/account-record workspace with AI-assisted parsing, deterministic validation, policy-gated ledger commit, and source-backed money views. MVP has no hosted service/backend; Gmail, BYO AI, optional fixed FX rates, update checks, and future connectors are capability-scoped local-client connections controlled by the user. An optional paid CanCan AI relay is an allowed post-MVP direction only; it must reuse the provider-adapter/capability boundary and has no current runtime, account, payment, or entitlement implementation.

## Implementation state

- The repository contains a real pnpm workspace with a React/Vite desktop shell, Tauri/Rust runtime, pure TypeScript core and parser packages, a slice-owned SQLite repository/migration boundary, a reusable UI package, and a disposable desktop feasibility spike. Production packages do not import the spike.
- Root typecheck, production unit-test, Rust-check, web-build, Tauri debug-build, and combined verification commands are real. The application workflow runs repository preflight, the separate Rust security/data-integrity suite, and `pnpm verify` on macOS with the pinned toolchain; isolated spike tests remain in their owning workflows.
- The foundation UI has been visually inspected at desktop and narrow viewports. It does not yet invoke network, Vault, database, secret, connector, or privileged command behavior.
- The synthetic-core harness safely resets only marked test databases, reapplies its migration idempotently, validates deterministic structured proposals against coherent raw rows, and exercises review, source-backed balance observations, exact transfer reconciliation, atomic commit, immutable audit, idempotency, and bidirectional record/event navigation. It uses no live model, network, vault, or production document.
- The `desktop-feasibility` slice is complete. ADR 0001 is accepted after a real Tauri 2, SQLCipher/FTS5, authenticated-file-encryption, Argon2id wrapper, and macOS Keychain smoke test.
- The `app-foundation` and `vault-security-validation` slices are complete. The accepted version-1 envelope, KDF selection, Keychain scope, memory-only viewer boundary, deletion recovery, and restore locator switch have reproducible macOS `arm64` evidence before production Vault implementation begins.
- The `synthetic-core-flow` slice is complete. Its schema and repository cover only exercised synthetic queries. `vault-manual-import` is now in progress: the production Rust host includes the canonical migrations, opens SQLCipher with a context-separated raw database subkey, writes `CCENV001` files, and coordinates exact-hash import outcomes with store-open/rollback recovery logic. App startup now owns a Rust-only locked/unlocked Vault session and exposes only status/create/unlock/lock commands; the accepted Argon2id password wrapper recovers the master key without returning it to the renderer. Creation prepares and syncs an inactive candidate before rename, rolls back handled activation failures, and can reopen an activated Vault after restart. An abrupt pre-activation process crash may leave an inactive `.vault-create-*` directory; automatic cleanup is deferred until CanCan has a cross-process ownership/locking protocol rather than risking deletion beneath a second app process. Host-owned file selection/import/list commands, parser, Source/Documents UI, viewer, Keychain remembering, recovery-file action, system/inactivity lock hooks, statement-password, and deletion wiring remain in this slice.

## Documentation state

- `docs/specs/README.md` is the only maintained canonical spec index.
- `docs/STRUCTURE.md` is the only source contract and conflict protocol.
- ADR authority follows each ADR's explicit status; ADR 0001, ADR 0002, and ADR 0003 are `Accepted`.
- `docs/alignment-temp/alignment-progress.md` is the only active unresolved decision register.
- `docs/specs/0018-app-updates-open-source-release.md` owns the accepted open-source GitHub release/update direction and its remaining blockers.
- Public launch uses a static Cloudflare Pages surface plus GitHub-native releases, discussions, issue forms, pull requests, and security reporting. Accounts, domain, identity, license, signing keys, and public contacts are not configured yet.
- MVP excludes analytics and behavioral telemetry.
- `.agents/scripts/agent-preflight.sh` provides deterministic gates.
- Project-scoped explorer, implementer, tester, and reviewer bindings pin their models and reasoning effort under `.codex/agents/`; explorer/reviewer default to read-only, tester defaults to workspace-write, and implementer has no repo-local sandbox default. Main-agent permissions remain user/session-owned, and parent live permissions can supersede every child default, so no-edit review remains a workflow/independence contract. Preflight rejects config drift but does not claim to validate future runtime sandbox selection.
- `docs/agent/implementation-slices.md` and `.agents/scripts/context-for-slice.sh` provide the machine-checked app implementation order and minimal per-slice context.
- `.agents/docs-semantic-review.md` defines the independent semantic gate and its complete change-scope trigger.
- The parser evidence contract is accepted: extraction/OCR produce job-scoped observations, a mandatory AI normalizer produces one structured proposal shape, and each record persists one bounded validated raw source object plus a validation summary rather than a field-evidence graph. Dates do not receive invented timezones, and Debit/Credit remains separate from signed source-account balance movement.
- Database migrations grow by vertical slice. The synthetic core keeps many-to-many `match_edges` because two bank-side records linking through one canonical transfer event is a core query, while speculative evidence-reference tables, identifier machinery, indexes, and fixed benchmark sizes are excluded.
- The `document-normalizer-runtime` evidence slice is complete. Single-pass structured normalization is selected for initial implementation in a Tauri-controlled bundled Node sidecar; ToolLoopAgent and Pi Agent Core remain unselected until qualification fixtures demonstrate a material advantage. Full Pi Coding Agent is excluded, the sidecar is not treated as a sandbox, and users install no separate runtime.
- The `vault-security-validation` evidence slice is complete. Its merged macOS CI gate and independent reviews accepted `CCENV001` version 1, both versioned Argon2id wrappers and the 750 ms selection rule, source-scoped Keychain password behavior, memory-only Core Graphics PDF rendering, tombstone-first deletion recovery, and inactive-Vault atomic locator switching. The `vault-manual-import` slice is in progress.
- Phase 1 supports macOS `arm64` and `x86_64`; Windows is Phase 2. Current end-to-end desktop, Vault, Keychain, and sidecar evidence exists only on `arm64`. Intel setup routing is test-covered, but a real `x86_64` build/runtime/security pass is still required before support can ship.
- The evidence-lifecycle/security grill is complete. `source_documents` also owns the encrypted-file registry/tombstone in MVP; there is no separate `vault_files` table. Source-file deletion retains records and ledger navigation, normal viewing renders in memory inside CanCan, and plaintext leaves the Vault only through explicit `Save a copy`.
- Vault password wrappers use versioned Argon2id profiles, with the RFC 9106 64 MiB profile preferred inside a 750 ms supported-Mac budget and the OWASP 19 MiB minimum as the only accepted fallback. Keychain keeps the normal remembered unlock path fast. One optional PDF password per Money Source is also stored in Keychain and excluded from backup.
- New-device restore validates a new Vault before atomic switch, restores non-secret data, and opens a resumable Setup Checklist for Gmail, AI, statement passwords, remembered unlock, and backup target.
- Recovery-file saving may be deferred during initial setup; the app must keep the recovery action visible and accurately show that recovery is not yet configured.
- The accepted visual direction is `Precision Vaultpunk — Obsidian Spine + Light Ledger`. Its version-1 palette, Geologica/Martian Mono roles, precision-motion rhythm, anti-Material/anti-vibe-code guardrails, selected mock, and rejected alternatives are recorded in the canonical design specs and `resources/design/vault-archive-2026-07/`. No redesigned product UI is implemented in this checkpoint.

The old broad numbered product-doc layer and duplicated alignment/harness projections have been removed. Do not recreate them.

## Immediate next work

Close the Vault-session command checkpoint without adding UI. After review and merge, continue `vault-manual-import` with host-owned file selection plus import/list commands, then normalize a synthetic/redacted statement through the selected mock sidecar boundary and implement Source/Documents against the accepted design contract. Viewer, Keychain remembering, recovery-file action, system/inactivity lock hooks, statement-password, and deletion remain later work inside this slice. Do not broaden into Gmail OAuth, public release, backup scheduling, or real `x86_64` qualification. A real `x86_64` build/runtime/security pass remains mandatory in `backup-release` before Phase 1 ships. Run the second public-launch grill after the review/ledger UI and before public OAuth/release work.

## External setup checkpoints

No user-owned external account or credential blocks `vault-manual-import`. CI and deterministic development use synthetic/redacted documents and a mock model until the local storage, job, and sidecar boundaries are ready.

- Before the first opt-in live-AI smoke test, the user chooses a supported AI provider, creates their own API key, and enters it through CanCan setup or an untracked local override. The key must never enter Git, fixtures, logs, or CI. Provider choice and the exact disclosure remain blocked by the AI-consent work; do not request a key earlier.
- Before live Gmail development, the user creates a separate Google Cloud development/test project, enables Gmail API, configures a Desktop OAuth client, and adds explicit test users. Mocked Gmail work may precede this.
- Before public Gmail availability, the user must provide the public app identity, domain ownership, privacy/support contacts, production Google Cloud project, and verification submission materials. Restricted-scope verification is a later release workstream and should not be started before the running data flow and disclosures are reviewable.

Do not implement behavior covered by an active blocker. A ready slice may use a spec for its explicitly unblocked outcome without implementing that spec's blocked release, security, privacy, or product behavior; the generated slice context is the executable boundary.

## Current scope guardrail

Do not start API trading, payments, bill pay, mobile sync, full budgeting, tax reporting, autonomous financial advice, or external market-data/FX fetching before the core evidence/reconciliation loop works and the relevant canonical spec changes.
