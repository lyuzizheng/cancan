# Grill Backlog

This file stores prioritized questions to ask the user. Move answered decisions into `alignment-progress.md` and canonical specs or ADRs.

## Batch 1: Visual Design and Command Center

1. What exact warm + green palette direction should CanCan use?
2. Should the warm background be nearly white, light clay, light mint, or neutral graphite-on-white?
3. What green should mean: money/growth, synced/safe, or primary action?
4. What accent should represent review/pending: amber, lime, or another warm tone?
5. Should AI insight appear as a right rail, inline card, chat drawer, or command palette?
6. How much animation is appropriate on Command Center after onboarding?
7. Which chart types are allowed in MVP: sparklines, bar charts, area charts, no charts?
8. Should DaisyUI/Hero UI be used directly, or only as inspiration?

## Batch 2: Ledger, Assets, Trades, Valuation

1. Should trades use only ledger legs or also specialized trade tables?
2. How should stock positions be derived: from trades, snapshots, or both?
3. Which source-provided position and valuation snapshots should be trusted/displayed before broker APIs?
4. How should realized/unrealized gains be represented?
5. Should insurance policy value be treated as asset, special instrument, or note-only?
6. How should source-provided or implied FX facts be stored and timestamped?
7. How should liabilities display alongside assets?
8. How should stale valuations affect Money Overview confidence or review state?

## Batch 3: Reconciliation and Review

1. What is default auto-commit policy?
2. What confidence threshold is required for safe standalone purchase auto-commit?
3. Should exact duplicate source documents be silently deduped or shown?
4. How does user undo a wrong commit?
5. How does user correct a wrong account mapping?
6. How should one-to-many matches be reviewed?
7. How should partial matches be reviewed?
8. Should confirmed matching behavior be learned for future suggestions?

## Batch 4: Evidence Library

1. Which file types are supported first?
2. What metadata can user edit?
3. Can a document be reclassified manually?
4. How does the Library show raw evidence vs derived artifacts?
5. How does user inspect native text, OCR text, parsed records, validation output, and ledger links?
6. What is the duplicate document UX?
7. How long should raw OCR outputs be retained?
8. Should Library support full-text search across extracted text?

## Batch 5: Security and Backup

1. Should file vault objects be individually encrypted in addition to SQLCipher?
2. How does restore handle schema migration in detail?
3. What export formats are required for portability?
4. Should local logs redact all amounts/descriptions by default?
5. Should cloud AI consent be per provider, per source, or per document?
6. What is acceptable failure mode if backup fails?
7. Should backups run automatically or only manually in MVP?
8. Should backup status appear in Command Center?

## Batch 6: Testing, Build, and AI Coding Agent Loop

1. Are redacted fixtures allowed in repo?
2. What integration tests must pass before a feature is considered done?
3. What command resets the test database?
4. How should LLM-dependent tests be deterministic?
5. What build/package command should the agent run?
6. Should repo-local `.agents/` skills/workflows be created to standardize browser/computer-use inspection?
7. When should the agent stop and ask the user?
8. What progress docs must be updated after every coding slice?
