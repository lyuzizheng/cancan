# Progress Log

Use this file to keep future AI coding agents oriented. Add a dated entry whenever product decisions, implementation scope, or architecture assumptions change.

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

### Next

- Produce detailed schema docs or migrations for the core domain tables.
- Define Gmail rule model and OAuth/security flow.
- Define parser JSON schemas and golden fixture policy.
- Produce a Command Center wireframe/spec before UI coding.
