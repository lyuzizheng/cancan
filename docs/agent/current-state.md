# Current State

Last updated: 2026-07-09

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

## Documentation state

`docs/specs/` is now the canonical implementation source of truth.

The old numbered docs layer (`00-product-vision.md` through `11-open-questions.md`) has been removed to avoid duplicate/conflicting guidance. Use `docs/README.md`, `docs/STRUCTURE.md`, this file, and the relevant specs instead.

## Current MVP decisions

- Manual import is a test harness and fallback.
- Supported providers are fixed/product-defined, not arbitrary user-created integrations.
- Initial provider scope: DBS bank, DBS credit card, UOB bank, UOB credit card, Wise PDF/CSV/export.
- Gmail read-only collection is part of MVP success.
- Gmail MVP uses Desktop OAuth Authorization Code Flow + PKCE + loopback redirect, then calls Gmail API locally.
- Gmail scope is read-only; allowed data includes metadata, message body when needed, and attachments.
- Gmail storage follows minimum storage: metadata + attachments by default, not full email body unless needed/enabled.
- Gmail sync should support app startup, wake/resume, manual refresh, and configurable polling; no backend Pub/Sub webhook for MVP.
- Gmail rule UX should be guided at the top and expert-query editable below.
- Password-protected statement PDFs must be detected and unlocked locally; saved statement passwords are optional and must live in OS secret storage, not plain SQLite.
- User manually creates Money Sources and child containers/accounts.
- Parser/AI maps documents to existing child containers/accounts or creates review suggestions.
- A Money Source represents a user-recognized institution/platform such as DBS, UOB, Wise, Moomoo, or Manulife, not a currency or asset type.
- MVP does not ask for base currency.
- Command Center should use Money Overview / Source Overview language, not default Net Worth.
- Native PDF extraction and OCR output should both be preserved.
- AI-assisted parsing is core from early phases.
- App may offer a default AI path, but should strongly prompt bring-your-own AI provider/key.
- Vercel AI SDK may be used for provider routing and structured generation.
- CanCan is fact-based: do not fetch market prices or external FX rates in MVP.
- Multi-currency display is preferred over invented conversion.
- Source/provider snapshots are current facts; parsed events explain changes.
- No tax-grade realized gains or lot accounting in MVP.
- Specialized trades table is deferred; preserve trade details in external records and represent canonical facts through ledger events/legs/snapshots.
- Low-risk standalone purchases may auto-commit after deterministic validation and threshold checks.
- Transfers, repayments, top-ups, FX conversions, broker deposits, and ambiguous links default to review.
- SQLCipher/equivalent encryption is required from v1.
- Use hand-written SQL migrations and typed repository functions. Do not use Prisma.
- SQL performance matters: add indexes deliberately and benchmark important queries.
- JSON fields are acceptable for provider-specific/evolving metadata when not hiding core query dimensions.
- Command Center should use left sidebar + main body, modern Warm Off-White + green polished finance app style, and future AI Assistant entry.
- Command Center should show user-facing facts, source activity, snapshot timestamps, AI insight, and review status without noisy metrics/tags.
- Money source types need tailored stats/display modes.
- Money Flow should preserve backend graph capability, but first UI can be chain-first.
- Backup should support generic folder first, with iCloud as a possible folder target, and must include version compatibility metadata.
- Backups do not include OAuth/API secrets, AI keys, vault key material, or statement PDF passwords by default.
- Repo-local `.agents/` workflows/skills exist for orientation, role routing, design grill, implementation, review, testing simulation, architecture refinement, UI refinement, and docs checks. Real app build/test/UI commands should be wired once application code exists.

## Current alignment work

A temporary alignment workspace exists in `docs/alignment-temp/`.

Purpose:

```text
break down the full product lifecycle
track aligned / partial / unresolved decisions
run structured grill-me discussion rounds
move stable decisions into canonical specs or ADRs
remove temp workspace after alignment is complete
```

## Immediate next design tasks

1. Align job engine and error model.
2. Align testing/fixtures and AI agent automation gates.
3. Align Evidence Library detail UX.
4. Decide exact design token values and whether to generate a Figma prototype.
5. Continue deleting or rewriting temporary alignment files once stable decisions move into specs.

## Do not start yet

Do not build API trading, payment, bill pay, mobile sync, full budgeting, tax reporting, autonomous financial advice, or external market-data/FX fetching before the core evidence/reconciliation loop works.
