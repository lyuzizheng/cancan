# Current State

Last updated: 2026-07-13

## Product phase

CanCan is at the boundary between documentation/design alignment and app foundation. Production application code has not started.

The current product target is a Gmail-first local financial evidence vault and polished personal finance/account-record workspace with AI-assisted parsing, deterministic validation, policy-gated ledger commit, and source-backed money views. CanCan has no hosted service/backend; Gmail, BYO AI, optional fixed FX rates, update checks, and future connectors are capability-scoped local-client connections controlled by the user.

## Implementation state

- The repository contains documentation, the coding-agent harness, and a disposable desktop feasibility spike. The spike is evidence, not production application code.
- No production app packages, migrations, runtime commands, build scripts, or UI implementation exist yet.
- App-specific test, DB reset, package, and visual-inspection commands must be added only after their real implementations exist.
- The `desktop-feasibility` slice is complete. ADR 0001 is accepted after a real Tauri 2, SQLCipher/FTS5, authenticated-file-encryption, Argon2id wrapper, and macOS Keychain smoke test.
- The `app-foundation` slice is in progress. Its reproducible macOS toolchain/setup subpart is complete; production workspace packages, typecheck/unit/build commands, and CI remain next. Exact production cryptographic formats and restore behavior remain blocked.

## Documentation state

- `docs/specs/README.md` is the only maintained canonical spec index.
- `docs/STRUCTURE.md` is the only source contract and conflict protocol.
- ADR authority follows each ADR's explicit status; ADR 0001 and ADR 0002 are `Accepted`.
- `docs/alignment-temp/alignment-progress.md` is the only active unresolved decision register.
- `docs/specs/0018-app-updates-open-source-release.md` owns the accepted open-source GitHub release/update direction and its remaining blockers.
- Public launch uses a static Cloudflare Pages surface plus GitHub-native releases, discussions, issue forms, pull requests, and security reporting. Accounts, domain, identity, license, signing keys, and public contacts are not configured yet.
- MVP excludes analytics and behavioral telemetry.
- `.agents/scripts/agent-preflight.sh` provides deterministic gates.
- `docs/agent/implementation-slices.md` and `.agents/scripts/context-for-slice.sh` provide the machine-checked app implementation order and minimal per-slice context.
- `.agents/docs-semantic-review.md` defines the independent semantic gate for docs and harness changes.

The old broad numbered product-doc layer and duplicated alignment/harness projections have been removed. Do not recreate them.

## Immediate next work

Continue `app-foundation` with the production workspace/package skeleton and real typecheck, unit-test, desktop-build, and CI commands; then implement `synthetic-core-flow`. Resume broad design grilling after the synthetic flow and before the vault/manual-import slice. Run a second public-launch grill after the review/ledger UI and before public OAuth/release work.

Do not implement behavior covered by an active blocker. A ready slice may use a spec for its explicitly unblocked outcome without implementing that spec's blocked release, security, privacy, or product behavior; the generated slice context is the executable boundary.

## Current scope guardrail

Do not start API trading, payments, bill pay, mobile sync, full budgeting, tax reporting, autonomous financial advice, or external market-data/FX fetching before the core evidence/reconciliation loop works and the relevant canonical spec changes.
