# Progress Log

Use this file to keep future AI coding agents oriented. Add a dated entry whenever product decisions, implementation scope, or architecture assumptions change.

## 2026-07-13

### Completed

- Completed `app-foundation`: added the production pnpm workspace, React/Vite desktop shell, Tauri/Rust runtime, empty core/database boundaries, reusable UI package, exact dependency pins, and frozen JavaScript/Rust lockfiles without adding product, network, vault, database, or secret behavior.
- Verified current upstream package releases on 2026-07-13 and pinned React 19.2.7, TypeScript 7.0.2, Vite 8.1.4, Vitest 4.1.10, Tauri CLI 2.11.4, Tauri crate 2.11.5, and tauri-build 2.6.3. The pnpm workspace keeps a seven-day maturity policy with exact/pattern exceptions only for the newly released pinned TypeScript, Vite, and matching Node type packages.
- Added real root `typecheck`, unit-test, Rust fmt/clippy, web-build, Tauri debug-build, and combined `verify` commands. Added a macOS application workflow that uses the pinned toolchain, frozen install, repository preflight, and the same root gate.
- Added application-CI structural gates and fault injections, bringing the harness self-test to 31 detected faults. The gate rejects removal of `pnpm verify`, either setup verification path, or unlocked production Cargo resolution. Updated setup to verify both the production application and the still-relevant desktop feasibility evidence while keeping the spike's nested pnpm/Cargo lockfiles isolated and frozen.
- Ran test-first UI work from a missing `AppShell` failure to a passing deterministic render test, then passed `pnpm verify`. Playwright desktop and 390px checks showed the neutral local-first shell correctly with zero console errors or warnings after adding its favicon.
- Marked `app-foundation` complete. `synthetic-core-flow` remains blocked only by the focused `Parser evidence contract`; no parser, financial, storage, or security behavior was inferred.
- Added `./scripts/setup-dev.sh` as an idempotent, Homebrew-free, no-`sudo` macOS setup. Apple Silicon is verified end to end; Intel archive routing is deterministically tested but awaits a real hardware run. It checksum-verifies the pinned official Node binary, installs pinned Corepack/pnpm and rustup/Rust with clippy/rustfmt, bootstraps frozen lockfiles, and runs preflight plus production application and desktop feasibility gates.
- Verified the current supported toolchain pins on 2026-07-13: latest-LTS Node.js 24.18.0, plus latest-stable Corepack 0.35.0, pnpm 11.12.0, Rust 1.97.0, and project-local Tauri CLI 2.11.4.
- Added deterministic setup tests for architecture selection, checksum rejection, pin consistency, unsupported OS handling, missing Command Line Tools UX, foreign tool-path preservation, existing-`fnm` coexistence, rustup shell/global-default isolation, and idempotent shell-profile changes; wired them into preflight and docs CI.
- Repeated setup on the current macOS arm64 machine: the cold run completed the full Tauri/SQLCipher gate; later runs converged to the same exact versions, the login shell resolved every pin correctly even with existing `fnm`, and the profile PATH line remained exactly once.
- Repaired stale harness self-test assumptions left from the pre-feasibility slice state and added a fault injection proving setup-version drift is rejected.
- Completed a disposable macOS arm64 desktop feasibility spike: Tauri 2 built, bundled SQLCipher 4.14.0 and FTS5 worked together, wrong database keys failed, authenticated file tampering failed, password and recovery wrappers opened one master key, and macOS Keychain binary-secret write/read/delete passed.
- Accepted ADR 0001 for the Tauri/React/Rust/SQLite package boundary while keeping exact production cryptographic formats, temporary plaintext, cross-platform secret storage, backup/restore, and release signing blocked in their owning specs.
- Fixed two broad-grill checkpoints in the implementation plan: after the synthetic core flow and before vault/manual import, then after the review-ledger UI and before public OAuth/release work.
- Added the public project surface to the implementation sequence: a static Cloudflare Pages landing/privacy/security/docs/download site and GitHub-native Discussions, issue forms, PR, security, and release surfaces.
- Defined the public Gmail direction as a project-owned Desktop OAuth client with separate development/test credentials, local PKCE loopback authorization, and an explicit Google restricted-scope verification track.
- Expanded the classic open-source release plan with signed/notarized CI artifacts, checksums, SBOM/provenance evidence, immutable source tags, approved releases, updater-key custody, and developer account/setup requirements.

- Allowed every financial event type to auto-commit only when the user toggle is on, all record/provider/account gates pass, package-calibrated required fields are very-high confidence, and every affected native-unit reconciliation window closes exactly against source-backed snapshots.
- Kept mixed-statement behavior record-level: semantic-only ambiguities can remain in Review while eligible records commit after full arithmetic closure; missing or uncertain financial fields block the window.
- Finalized account identity fields and resolver order, one-time first-account confirmation, idempotent aliases, and audited rename/merge/archive behavior without rewriting committed ledger legs.
- Added cross-channel duplicate outcomes and an import completion summary that identifies files already present or already archived/removed without deciding deletion semantics.
- Expanded onboarding into a polished animated local-first/vault story with encryption/privacy explanation and a final enabled-capabilities review.
- Added `0018-app-updates-open-source-release.md` for semantic versions, public GitHub Releases, signed CI artifacts, forward compatibility, safe parser delivery, and conditional desktop updater integration.
- Clarified that CanCan has no hosted service/backend: Gmail, BYO AI, optional fixed FX rates, update checks, and future connectors are separately authorized local-client connections; MVP has no analytics or behavioral telemetry.
- Simplified user-facing security to vault password, optional `Remember on this Mac`, and one recovery file. Internal key separation remains hidden, backup adds no second password, and exact cryptographic/restore details move to feasibility validation.
- Added a machine-checked implementation sequence so coding agents load only the selected slice's specs/ADRs/blockers instead of all canonical specs.
- Added shared context and implementation-review packet generators for implementer/tester/reviewer parity, plus deterministic dependency/path/blocker/test/outcome validation.

### Next

- Resolve `Parser evidence contract`, implement `synthetic-core-flow`, then resume a grouped 5-10 question grill before touching real vault/manual-import behavior.
- Configure public domain/identity/accounts only when their owning slice is reached; resolve license/platform/signing/update-channel details before release automation.

## 2026-07-12

### Completed

- Accepted the AI advisor boundary: a qualified deterministic policy may commit eligible records, while AI confidence alone never grants ledger-write authority.
- Defined immutable committed events, posting versus observation classes, simple event-type invariants, reversal/replacement correction, commit idempotency, and atomic append-only audit.
- Defined source-backed initial balance anchors so first import does not assume zero or invent historical transactions.
- Defined many-to-many record/event allocations and consumer-first progressive disclosure for partial matches.
- Selected SHA-256 for exact file deduplication, separate semantic document/record identity, and versioned reparse/supersede behavior.
- Reframed the UI as a polished, future-facing personal finance and account-record product with presentation-ready views and low user mental load; sharing remains post-MVP.
- Defined Money Sources as user-configured provider roots for editable Gmail discovery, explicit imports, and future product-defined official API connectors; matching documents continue parsing when they reveal multiple child-account candidates.
- Standardized every provider/document parser as a versioned package containing classifier/prompt configuration, deterministic fingerprints and validators, account mapping, fixtures, qualification state, and implementation notes.
- Replaced global AI confidence thresholds with provider-package qualification: 100 labeled record cases, 20 confirmed shadow candidates, zero incorrect eligible outcomes, and qualification reset after parser/prompt/rule changes.
- Kept the broader `clear PDF` auto-commit direction partial until eligible financial event types and first-seen account commit behavior are decided; accepted one simple setting, quiet Recent Activity, reversal-backed Undo, and no per-record notifications.

### Next

- Continue the next unresolved P0 batch from `docs/alignment-temp/alignment-progress.md`.

## 2026-07-10

### Completed

- Changed design grill rounds to consistently ask 5-10 related questions, ordered by risk, instead of falling back to one-question-at-a-time interviews.
- Added `0017-evidence-documents-source-ux.md`: Evidence lives under Money Source detail rather than a dominant standalone Library section.
- Synchronized the spec index, Command Center navigation, current state, reading path, and active alignment register with `0017`.
- Marked unresolved Evidence Remove/reversal and encrypted original-file behavior as explicit implementation blockers instead of inventing product or security answers.
- Replaced the duplicated source-of-truth precedence list with one concern-based source contract in `docs/STRUCTURE.md`.
- Simplified `.agents/` by removing duplicate roles, product rules, placeholder plugin guidance, generic templates, brittle keyword routing, and static priority copies.
- Added a tool-neutral root `AGENTS.md`, bilingual intent routing, deterministic spec/index/link/skill/doc checks, shell syntax validation, and harness fault-injection self-tests.
- Added an independent semantic-review contract and review-packet generator for docs and harness changes.
- Added a docs-harness GitHub Action plus a contract check that protects its triggers, paths, permissions, and required commands.
- Consolidated temporary alignment material into one active unresolved decision register and deleted stale lifecycle/audit/backlog copies.

### Next

- Run the next design round from `docs/alignment-temp/alignment-progress.md`, starting with P0 implementation blockers.
- Wire real app commands into the harness only after application/package scripts exist.

## 2026-07-09

### Completed

- Pulled latest `main` and re-evaluated docs after the old numbered docs layer was removed.
- Updated temporary alignment docs so permanent homes point to canonical specs/current docs instead of deleted numbered docs.
- Reconciled remaining base-currency, default Net Worth, and future crypto/source valuation wording with `0013` and `0014`.
- Updated agent workflow/checklist references to use `docs/specs/` instead of the removed numbered docs layer.
- Created repo-local `.agents/` operating workspace with role routing, workflows, rules, skills, plugin guidance, templates, and deterministic docs/preflight scripts.
- Updated agent reading order and repo-agent workflow spec so future agents use `.agents/` without duplicating product truth from `docs/specs/`.
- Reviewed `.agents/` after creation and fixed grill workflow mismatch: CanCan design grill now asks focused batches of 5-10 questions by default, with one-question mode reserved for security, money correctness, irreversible data shape, or single blocking ambiguity.
- Updated `docs/agent/README.md` to explicitly describe the split between persistent project memory in `docs/agent/` and operating workflows in `.agents/`.
- Aligned job engine and error model in `docs/specs/0015-job-engine-error-model.md`.
- Recorded that jobs should be coarse-grained and user-meaningful, with internal step checkpoints instead of many tiny jobs.
- Added job engine to specs index and agent reading order.
- Aligned testing, fixtures, and AI agent automation gates in `docs/specs/0016-testing-fixtures-agent-gates.md`.
- Added `.gitignore` entries for `fixtures-private/`, local vault/test folders, and common generated output.
- Updated `.agents` testing workflow and testing rules for private/redacted/synthetic fixtures, deterministic LLM mocks, safe test DB reset, and UI visual inspection gates.

### Next

- Align Evidence Library detail UX.
- Decide exact design token values and whether to generate a Figma prototype.
- Align future optional estimated-total/network-valuation policy.

## 2026-07-08

### Completed

- Added implementation spec layer under `docs/specs/`.
- Recorded decisions that the app should use hand-written SQL migrations and typed repositories, not Prisma.
- Added SQL/index/benchmark policy and test database reset requirements.
- Recorded that Vercel AI SDK may be used for AI provider routing and structured generation.
- Defined Gmail rule UX as guided builder plus expert query preview/editing.
- Clarified Command Center layout: left sidebar + main body, polished asset-management/reconciliation app style.
- Added future AI Assistant direction: assistant accesses backend APIs/skills, not raw DB/files/secrets.
- Clarified Review UI should be simple and low-friction, with side-by-side detail only where needed.
- Clarified Money Flow should keep backend graph capability while first UI is chain-first.
- Added expectation that future AI coding agents can implement, test, build, inspect UI via browser/computer-use, reset database, and iterate.
- Added temporary alignment workspace under `docs/alignment-temp/` to break down full product lifecycle questions and track alignment progress.
- Added first-run/onboarding spec with product promise, fixed provider policy, bring-your-own AI direction, startup sequence, and interaction/motion requirements.
- Added design system spec with light-first, modern warm+green, technical/safe/premium direction.
- Added backup/restore/versioning spec with generic folder backup, manifest, compatibility rules, and restore behavior.
- Added agentic development workflow spec with required tests, DB reset, build/package, UI inspection, and docs update loop.
- Aligned Gmail integration: Desktop OAuth Authorization Code Flow + PKCE + loopback redirect; local token exchange; Gmail readonly; local Keychain token storage; local encrypted mail cache; polling sync; no CanCan server.
- Recorded Google restricted scope/OAuth verification risk for public release.
- Added markdown Command Center wireframe.
- Added visual design tokens spec for Warm Off-White + green semantic direction.
- Recorded component strategy: prefer Hero UI-style foundation with CanCan wrappers and centralized tokens.
- Added repo agent workflows spec for future `.agents/` rules/workflows/skills after real commands exist.
- Added ledger/assets/valuation spec: fact-based, no market price fetch, no external FX rates, multi-currency first, source snapshots as facts, no tax/lot accounting in MVP, trade table deferred.
- Added Money Overview/source taxonomy spec: no base currency in MVP, no default Net Worth, two-level source model, source/account/instrument taxonomy.
- Added support for password-protected statement PDFs: local unlock, optional secure save, secret references only in SQLite.
- Reworked documentation structure so `docs/specs/` is the canonical implementation source of truth.
- Removed old numbered docs layer (`00-product-vision.md` through `11-open-questions.md`) to prevent duplicate and conflicting product truth.
- Added `docs/STRUCTURE.md` and `docs/specs/README.md`.
- Updated AI agent reading order and source-of-truth hierarchy after cleanup.

### Next

- Align job engine and error model.
- Align testing/fixtures and AI agent automation gates.
- Align Evidence Library detail UX.
- Continue deleting or rewriting temporary alignment files once stable decisions move into specs.

## 2026-07-07

### Completed

- Created initial CanCan docs for product vision, architecture, domain model, AI parser, reconciliation, plugins, security, UI IA, technology decisions, roadmap, open questions, and ADRs.
- Clarified that CanCan is not a budgeting app. It is a local-first financial evidence vault and reconciliation console.
- Updated MVP definition: Gmail-first automation is required; manual import is only a test harness/fallback.
- Moved AI-assisted parsing earlier in the roadmap. AI normalization, duplicate recognition, and link explanation are core capabilities.
- Confirmed first provider scope should include DBS/UOB bank and credit-card statements plus Wise PDF/CSV/export.
- Confirmed user-created Money Sources and sub-accounts are the source of truth for accounts.
- Confirmed ledger events/legs are the canonical model, including transactions, trades, balance snapshots, and valuation snapshots.
- Added `docs/agent/` as a working memory and consistency system for future AI coding agents.
