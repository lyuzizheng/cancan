# Progress Log

Use this file to keep future AI coding agents oriented. Add a dated entry whenever product decisions, implementation scope, or architecture assumptions change.

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

### Next

- Decide exact design token values and whether to generate a Figma prototype.
- Align ledger details for trades, positions, valuations, and gains.
- Align job engine and error model.
- Align testing/fixtures and AI agent automation gates.

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
