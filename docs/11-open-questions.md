# 11. Open Questions

This file tracks decisions from product discussions. Keep it updated when decisions change.

## Resolved decisions

### Product and MVP

1. MVP success requires Gmail-first automation. Manual PDF import is a test harness and fallback, not the success condition.
2. The app should support user-configured Gmail search rules with start date and incremental scan.
3. Gmail rule UX should be guided builder plus expert Gmail query preview/editing.
4. Initial provider scope should include DBS bank statement, DBS credit card statement, UOB bank statement, UOB credit card statement, and Wise PDF/CSV/export.
5. The product is not primarily a budgeting app. It is a local financial evidence vault and reconciliation console.
6. Default base currency is SGD, configurable in vault settings. Native values must always be preserved.

### Parsing and AI

1. AI-assisted parsing is core, not a late optional feature.
2. PDF processing should use native text extraction and an OCR layer, preserving both outputs.
3. The LLM helps normalize records, classify event types, identify duplicates, and propose links.
4. Vercel AI SDK may be used behind CanCan-owned adapters.
5. Insert/commit must pass deterministic validation and policy gates.
6. Low-risk standalone purchases may auto-commit if validation and confidence thresholds pass.
7. Transfers, repayments, top-ups, FX conversions, broker deposits, and ambiguous links should default to review.

### Accounts and ledger

1. User manually creates Money Sources and sub-accounts first.
2. Parser/AI may suggest which source/sub-account a statement belongs to.
3. Credit card repayment is not spending. It reduces cash and liability; the original card purchase is the spending event.
4. Balance and valuation snapshots may enter the ledger model as snapshot events, but must not be counted as normal transactions.
5. `ledger_events` and `ledger_legs` are supported as the canonical financial model.
6. Money Flow should keep backend graph capability, but first UI can be chain-first.

### Security and platform

1. SQLCipher or equivalent encrypted database should be mandatory from v1.
2. Gmail should use official read-only OAuth/API, not computer-use browser automation for MVP.
3. AI providers are opt-in and must not receive secrets.
4. The app remains local-first with no CanCan-hosted backend for MVP.

### Engineering and docs

1. Use pure TypeScript core engine.
2. Use hand-written SQL migrations and typed repositories. Do not use Prisma.
3. Benchmark hot SQL paths and design indexes deliberately.
4. JSON fields are acceptable for provider-specific/evolving metadata, but not for core query dimensions.
5. Future AI coding agents should implement from `docs/specs/`, run tests, reset databases, build/package the app, inspect UI visually, and update docs/progress in the same loop.

### UI

1. Use a left sidebar + main body desktop app layout.
2. Command Center should feel like a polished asset-management and reconciliation app.
3. AI Assistant should be represented as future surface backed by narrow backend APIs/skills.
4. Review UI should avoid unnecessary complexity but provide enough evidence to prevent wrong reconciliation.

## Remaining product questions

1. What confidence threshold should allow standalone purchases to auto-commit?
2. Should the user be able to disable auto-commit entirely?
3. Should cash/physical wallet be excluded, hidden by default, or included as a manual source?
4. How much category/budget functionality is necessary after ledger/reconciliation works?
5. How should insurance premiums be classified by default: expense, asset transfer, or configurable per policy?

## Remaining data model questions

1. Should `match_edges` always support both external_record links and ledger_event links, or should pre-commit and post-commit links use separate tables?
2. Should trades use only generic ledger legs, or also specialized trade tables for brokerage reporting?
3. How should realized and unrealized gains be represented?
4. How should one-to-many matches be represented, such as one bank payment paying multiple card balances?
5. How should partial matches be represented?
6. What valuation source should be used for stock/crypto/fund base-currency conversion?

## Remaining parser questions

1. Which exact DBS/UOB/Wise sample should be the first golden fixture?
2. How should password-protected PDFs be handled in the UI?
3. Should OCR bounding boxes be stored for all documents or only when the parser needs them?
4. What validation failures should block staging versus allow staging with warning?
5. How should parser version upgrades compare old and new outputs?

## Remaining reconciliation questions

1. What confidence threshold is safe for auto-linking, if any?
2. Should known self-transfer relationships be learned from user confirmations?
3. How far apart can bank outflows and broker deposits be before they require manual review?
4. How should FX fees be detected and allocated?
5. How should payment app channel purchases be reported to avoid double-counting?

## Remaining UI questions

1. What exact visual theme/tokens should Command Center use?
2. What should the first empty-state onboarding path look like after vault creation?
3. Should Command Center prioritize review queue or asset snapshot above the fold?
4. How prominent should the AI Assistant be in MVP UI?
