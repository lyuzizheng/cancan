# Current State

Last updated: 2026-07-07

## Product state

CanCan is in documentation/design phase. Application code has not started.

The current product target is:

```text
Gmail-first local finance evidence automation
+ AI-assisted parsing and normalization
+ deterministic validation
+ review-first reconciliation
+ local encrypted ledger and asset view
```

## Current MVP decisions

- Manual import is a test harness and fallback.
- Gmail read-only collection is part of MVP success.
- User manually creates Money Sources and sub-accounts.
- Parser/AI maps documents to existing sub-accounts or creates review suggestions.
- Initial provider scope: DBS bank, DBS credit card, UOB bank, UOB credit card, Wise PDF/CSV/export.
- Native PDF extraction and OCR output should both be preserved.
- AI-assisted parsing is core from early phases.
- Low-risk standalone purchases may auto-commit after deterministic validation and threshold checks.
- Transfers, repayments, top-ups, FX conversions, broker deposits, and ambiguous links default to review.
- Default base currency is SGD, configurable.
- SQLCipher/equivalent encryption is required from v1.

## Immediate next design tasks

1. Define exact schemas for Money Source, Account, Source Document, Parse Run, External Record, Ledger Event, Ledger Leg, Match Edge, and Review Item.
2. Define Gmail search-rule UX and data model.
3. Define provider fixture policy and sample naming convention.
4. Define parser output JSON schemas and validation gates.
5. Define auto-commit threshold policy for standalone purchases.
6. Define Command Center first-screen wireframe.

## Do not start yet

Do not build API trading, payment, bill pay, mobile sync, full budgeting, tax reporting, or autonomous financial advice before the core evidence/reconciliation loop works.
