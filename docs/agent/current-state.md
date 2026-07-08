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
- Gmail read-only collection is part of MVP success.
- Gmail rule UX should be guided at the top and expert-query editable below.
- User manually creates Money Sources and sub-accounts.
- Parser/AI maps documents to existing sub-accounts or creates review suggestions.
- Initial provider scope: DBS bank, DBS credit card, UOB bank, UOB credit card, Wise PDF/CSV/export.
- Native PDF extraction and OCR output should both be preserved.
- AI-assisted parsing is core from early phases.
- Vercel AI SDK may be used for provider routing and structured generation.
- Low-risk standalone purchases may auto-commit after deterministic validation and threshold checks.
- Transfers, repayments, top-ups, FX conversions, broker deposits, and ambiguous links default to review.
- Default base currency is SGD, configurable.
- SQLCipher/equivalent encryption is required from v1.
- Use hand-written SQL migrations and typed repository functions. Do not use Prisma.
- SQL performance matters: add indexes deliberately and benchmark important queries.
- JSON fields are acceptable for provider-specific/evolving metadata when not hiding core query dimensions.
- Command Center should use left sidebar + main body, polished finance app style, and future AI Assistant entry.
- Money Flow should preserve backend graph capability, but first UI can be chain-first.

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

1. Work through `docs/alignment-temp/alignment-progress.md` from highest-risk unresolved areas.
2. Align first-run/startup sequence.
3. Align visual design system and Command Center UX.
4. Align database/ledger details for trades, positions, valuations, and gains.
5. Align backup/restore model.
6. Align testing/fixtures and AI agent automation gates.

## Do not start yet

Do not build API trading, payment, bill pay, mobile sync, full budgeting, tax reporting, or autonomous financial advice before the core evidence/reconciliation loop works.
