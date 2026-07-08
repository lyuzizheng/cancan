# Current State

Last updated: 2026-07-08

## Product state

CanCan is in documentation/design phase. Application code has not started.

The current product target is:

```text
Gmail-first local finance evidence automation
+ AI-assisted parsing and normalization
+ deterministic validation
+ review-first reconciliation
+ local encrypted ledger and asset view
+ future AI assistant backed by narrow backend APIs/skills
```

## Current MVP decisions

- Manual import is a test harness and fallback.
- Supported providers are fixed/product-defined, not arbitrary user-created integrations.
- Gmail read-only collection is part of MVP success.
- Gmail MVP uses Desktop OAuth Authorization Code Flow + PKCE + loopback redirect, then calls Gmail API locally.
- Gmail scope is read-only; allowed data includes metadata, message body when needed, and attachments.
- Gmail storage follows minimum storage: metadata + attachments by default, not full email body unless needed/enabled.
- Gmail sync should support app startup, wake/resume, manual refresh, and configurable polling; no backend Pub/Sub webhook for MVP.
- Gmail rule UX should be guided at the top and expert-query editable below.
- User manually creates Money Sources and sub-accounts.
- Parser/AI maps documents to existing sub-accounts or creates review suggestions.
- Initial provider scope: DBS bank, DBS credit card, UOB bank, UOB credit card, Wise PDF/CSV/export.
- Native PDF extraction and OCR output should both be preserved.
- AI-assisted parsing is core from early phases.
- App may offer a default AI path, but should strongly prompt bring-your-own AI provider/key.
- Vercel AI SDK may be used for provider routing and structured generation.
- Low-risk standalone purchases may auto-commit after deterministic validation and threshold checks.
- Transfers, repayments, top-ups, FX conversions, broker deposits, and ambiguous links default to review.
- Default base currency is SGD, configurable.
- SQLCipher/equivalent encryption is required from v1.
- Use hand-written SQL migrations and typed repository functions. Do not use Prisma.
- SQL performance matters: add indexes deliberately and benchmark important queries.
- JSON fields are acceptable for provider-specific/evolving metadata when not hiding core query dimensions.
- Command Center should use left sidebar + main body, modern Warm Off-White + green polished finance app style, and future AI Assistant entry.
- Command Center should be multi-dimensional: overview, money source flows, snapshot freshness, AI insight, and review status without excessive density.
- Money source types need tailored stats/display modes.
- Preferred component strategy is Hero UI-style app components plus CanCan-owned wrappers and centralized tokens; avoid default library look.
- Money Flow should preserve backend graph capability, but first UI can be chain-first.
- Backup should support generic folder first, with iCloud as a possible folder target, and must include version compatibility metadata.
- Repo-local `.agents/` workflows/skills should be created later once real build/test/UI commands exist.

## Current alignment work

A temporary alignment workspace exists in `docs/alignment-temp/`.

Purpose:

```text
break down the full product lifecycle
track aligned / partial / unresolved decisions
run structured grill-me discussion rounds
move stable decisions into permanent docs/specs
remove temp workspace after alignment is complete
```

## Immediate next design tasks

1. Decide exact design token values and whether to generate a Figma prototype.
2. Align database/ledger details for trades, positions, valuations, and gains.
3. Align job engine and error model.
4. Align testing/fixtures and AI agent automation gates.
5. Align Evidence Library detail UX.

## Do not start yet

Do not build API trading, payment, bill pay, mobile sync, full budgeting, tax reporting, or autonomous financial advice before the core evidence/reconciliation loop works.
