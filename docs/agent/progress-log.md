# Progress Log

Use this file to keep future AI coding agents oriented. Add a dated entry whenever product decisions, implementation scope, or architecture assumptions change.

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
