# Grill Backlog

This file stores prioritized questions to ask the user. Move answered decisions into `alignment-progress.md` and permanent docs/specs.

## Batch 1: App Lifecycle and First-Run

1. What exactly happens on first launch before a vault exists?
2. Is vault password/key setup mandatory before any UI is visible?
3. Can user skip Gmail setup and only use manual import temporarily?
4. Can user skip AI provider setup and use deterministic parsing only?
5. What is the first successful empty-state path: create source, import sample, or connect Gmail?
6. How should app resume unfinished jobs after unlock?
7. What should happen if migrations fail?
8. What should happen if vault unlock fails repeatedly?

## Batch 2: Visual Design and Command Center

1. Should MVP be light theme first, dark theme first, or adaptive?
2. What brand color direction should CanCan use?
3. Is the visual target closer to Linear, Bloomberg, Apple Finance, Monarch, or something else?
4. How dense should tables be by default?
5. Where does AI Assistant appear: sidebar page, right drawer, command palette, or all later?
6. Should Command Center prioritize review queue or net worth snapshot above the fold?
7. Which charts are necessary in MVP, if any?
8. What should the empty Command Center look like before data exists?

## Batch 3: Ledger, Assets, Trades, Valuation

1. Should trades use only ledger legs or also specialized trade tables?
2. How should stock positions be derived: from trades, snapshots, or both?
3. How should current market values be updated without broker API?
4. How should realized/unrealized gains be represented?
5. Should insurance policy value be treated as asset, special instrument, or note-only?
6. How should FX rates be sourced and timestamped?
7. How should liabilities display alongside assets?
8. How should stale valuations affect net worth confidence?

## Batch 4: Reconciliation and Review

1. What is default auto-commit policy?
2. What confidence threshold is required for safe standalone purchase auto-commit?
3. Should exact duplicate source documents be silently deduped or shown?
4. How does user undo a wrong commit?
5. How does user correct a wrong account mapping?
6. How should one-to-many matches be reviewed?
7. How should partial matches be reviewed?
8. Should confirmed matching behavior be learned for future suggestions?

## Batch 5: Gmail and Evidence Library

1. Should CanCan store email body text or only metadata and attachments?
2. Can one Gmail rule map to multiple providers?
3. Should user be able to preview matching emails before import?
4. How are password-protected PDFs handled?
5. Should duplicate attachments appear as duplicate evidence or be hidden?
6. Can user manually reclassify a source document?
7. How long should raw OCR outputs be retained?
8. Should Library support full-text search across extracted text?

## Batch 6: Security and Backup

1. Backup target first: iCloud only, generic folder only, or both?
2. Should backup include secrets? Current recommendation: no.
3. Should file vault objects be individually encrypted in addition to SQLCipher?
4. How does restore handle schema migration?
5. What export formats are required for portability?
6. Should local logs redact all amounts/descriptions by default?
7. Should cloud AI consent be per provider, per source, or per document?
8. What is acceptable failure mode if backup fails?

## Batch 7: Testing, Build, and AI Coding Agent Loop

1. Are redacted fixtures allowed in repo?
2. What integration tests must pass before a feature is considered done?
3. What command resets the test database?
4. How should LLM-dependent tests be deterministic?
5. What build/package command should the agent run?
6. Should the agent always use browser/computer-use for UI changes?
7. When should the agent stop and ask the user?
8. What progress docs must be updated after every coding slice?
