# Current State

Last updated: 2026-07-14

## Product phase

CanCan has crossed from documentation/design alignment into production implementation. The app foundation and synthetic core flow are complete; production vault, live-document normalization, and user-facing financial behavior have not started.

The current product target is a Gmail-first local financial evidence vault and polished personal finance/account-record workspace with AI-assisted parsing, deterministic validation, policy-gated ledger commit, and source-backed money views. CanCan has no hosted service/backend; Gmail, BYO AI, optional fixed FX rates, update checks, and future connectors are capability-scoped local-client connections controlled by the user.

## Implementation state

- The repository contains a real pnpm workspace with a React/Vite desktop shell, Tauri/Rust runtime, pure TypeScript core and parser packages, a slice-owned SQLite repository/migration boundary, a reusable UI package, and a disposable desktop feasibility spike. Production packages do not import the spike.
- Root typecheck, unit-test, Rust-check, web-build, Tauri debug-build, and combined verification commands are real. The application workflow runs the same gate on macOS with the pinned toolchain.
- The foundation UI has been visually inspected at desktop and narrow viewports. It has no network, vault, database, secret, connector, or privileged command behavior.
- The synthetic-core harness safely resets only marked test databases, reapplies its migration idempotently, validates deterministic structured proposals against coherent raw rows, and exercises review, source-backed balance observations, exact transfer reconciliation, atomic commit, immutable audit, idempotency, and bidirectional record/event navigation. It uses no live model, network, vault, or production document.
- The `desktop-feasibility` slice is complete. ADR 0001 is accepted after a real Tauri 2, SQLCipher/FTS5, authenticated-file-encryption, Argon2id wrapper, and macOS Keychain smoke test.
- The `app-foundation` slice is complete. Exact production cryptographic formats and restore behavior remain blocked in later owning slices.
- The `synthetic-core-flow` slice is complete. Its schema and repository cover only exercised synthetic queries; production document storage, reparse lifecycle, duplicate import, and runtime selection remain in later slices.

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
- The parser evidence contract is accepted: extraction/OCR produce job-scoped observations, a mandatory AI normalizer produces one structured proposal shape, and each record persists one bounded validated raw source object plus a validation summary rather than a field-evidence graph. Dates do not receive invented timezones, and Debit/Credit remains separate from signed source-account balance movement.
- Database migrations grow by vertical slice. The synthetic core keeps many-to-many `match_edges` because two bank-side records linking through one canonical transfer event is a core query, while speculative evidence-reference tables, identifier machinery, indexes, and fixed benchmark sizes are excluded.
- A bounded document agent may implement the normalizer with seven fixed parser tools, but ToolLoopAgent versus Pi Agent Core and the bundled Node/Tauri execution path remain an evidence-only runtime decision. Full Pi Coding Agent is excluded; users will not install a sandbox runtime.

The old broad numbered product-doc layer and duplicated alignment/harness projections have been removed. Do not recreate them.

## Immediate next work

`document-normalizer-runtime` is now the immediate evidence-only slice. Compare ToolLoopAgent, Pi Agent Core, and the simpler single-pass baseline through the shared deterministic contract, then prove or reject a bundled Node/Tauri execution boundary. Do not connect a live model or promote the spike into production while collecting this evidence.

After the runtime evidence is recorded, run the planned grouped 5-10 question grill before vault/manual-import work. Run a second public-launch grill after the review/ledger UI and before public OAuth/release work.

Do not implement behavior covered by an active blocker. A ready slice may use a spec for its explicitly unblocked outcome without implementing that spec's blocked release, security, privacy, or product behavior; the generated slice context is the executable boundary.

## Current scope guardrail

Do not start API trading, payments, bill pay, mobile sync, full budgeting, tax reporting, autonomous financial advice, or external market-data/FX fetching before the core evidence/reconciliation loop works and the relevant canonical spec changes.
