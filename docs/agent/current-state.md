# Current State

Last updated: 2026-07-15

## Product phase

CanCan has crossed from documentation/design alignment into production implementation. The app foundation and synthetic core flow are complete; production vault, live-document normalization, and user-facing financial behavior have not started.

The current product target is a Gmail-first local financial evidence vault and polished personal finance/account-record workspace with AI-assisted parsing, deterministic validation, policy-gated ledger commit, and source-backed money views. CanCan has no hosted service/backend; Gmail, BYO AI, optional fixed FX rates, update checks, and future connectors are capability-scoped local-client connections controlled by the user.

## Implementation state

- The repository contains a real pnpm workspace with a React/Vite desktop shell, Tauri/Rust runtime, pure TypeScript core and parser packages, a slice-owned SQLite repository/migration boundary, a reusable UI package, and a disposable desktop feasibility spike. Production packages do not import the spike.
- Root typecheck, unit-test, Rust-check, web-build, Tauri debug-build, and combined verification commands are real. The application workflow runs the same gate on macOS with the pinned toolchain.
- The foundation UI has been visually inspected at desktop and narrow viewports. It has no network, vault, database, secret, connector, or privileged command behavior.
- The synthetic-core harness safely resets only marked test databases, reapplies its migration idempotently, validates deterministic structured proposals against coherent raw rows, and exercises review, source-backed balance observations, exact transfer reconciliation, atomic commit, immutable audit, idempotency, and bidirectional record/event navigation. It uses no live model, network, vault, or production document.
- The `desktop-feasibility` slice is complete. ADR 0001 is accepted after a real Tauri 2, SQLCipher/FTS5, authenticated-file-encryption, Argon2id wrapper, and macOS Keychain smoke test.
- The `app-foundation` slice is complete. Accepted production security behavior now has a bounded `vault-security-validation` evidence slice before real Vault data is accepted.
- The `synthetic-core-flow` slice is complete. Its schema and repository cover only exercised synthetic queries; production document storage, reparse lifecycle, duplicate import, and selected-runtime integration remain in later slices.

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
- Phase 1 supports macOS `arm64` and `x86_64`; Windows is Phase 2. Current end-to-end desktop, Vault, Keychain, and sidecar evidence exists only on `arm64`. Intel setup routing is test-covered, but a real `x86_64` build/runtime/security pass is still required before support can ship.
- The evidence-lifecycle/security grill is complete. `source_documents` also owns the encrypted-file registry/tombstone in MVP; there is no separate `vault_files` table. Source-file deletion retains records and ledger navigation, normal viewing renders in memory inside CanCan, and plaintext leaves the Vault only through explicit `Save a copy`.
- Vault password wrappers use versioned Argon2id profiles, with the RFC 9106 64 MiB profile preferred inside a 750 ms supported-Mac budget and the OWASP 19 MiB minimum as the only accepted fallback. Keychain keeps the normal remembered unlock path fast. One optional PDF password per Money Source is also stored in Keychain and excluded from backup.
- New-device restore validates a new Vault before atomic switch, restores non-secret data, and opens a resumable Setup Checklist for Gmail, AI, statement passwords, remembered unlock, and backup target.

The old broad numbered product-doc layer and duplicated alignment/harness projections have been removed. Do not recreate them.

## Immediate next work

Run the bounded `vault-security-validation` evidence slice next on the current `arm64` machine. It must prove the accepted KDF profiles and unlock budget, architecture-portable authenticated envelope fixtures, Keychain behavior, in-memory/no-temp-file viewing, tombstone deletion crash recovery, and restore-to-new-path atomic switching before `vault-manual-import` becomes ready. A real `x86_64` build/runtime/security pass remains mandatory in `backup-release` before Phase 1 ships, but does not block architecture-neutral feature implementation. Run the second public-launch grill after the review/ledger UI and before public OAuth/release work.

Do not implement behavior covered by an active blocker. A ready slice may use a spec for its explicitly unblocked outcome without implementing that spec's blocked release, security, privacy, or product behavior; the generated slice context is the executable boundary.

## Current scope guardrail

Do not start API trading, payments, bill pay, mobile sync, full budgeting, tax reporting, autonomous financial advice, or external market-data/FX fetching before the core evidence/reconciliation loop works and the relevant canonical spec changes.
