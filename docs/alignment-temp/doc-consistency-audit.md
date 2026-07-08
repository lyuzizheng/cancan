# Temporary Doc Consistency Audit

Status: active temporary audit.

Purpose: track duplicate/conflicting documentation while the product spec is still being aligned. Delete this file when the canonical docs are clean and stable.

## Resolved in Canonical Docs

### Old numbered docs layer

Resolution:

```text
Removed docs/00-product-vision.md through docs/11-open-questions.md from the canonical docs tree.
Canonical implementation truth now lives in docs/specs/.
Project-level navigation lives in docs/README.md and docs/STRUCTURE.md.
AI memory lives in docs/agent/.
```

### Base currency

Resolution:

```text
MVP does not ask for base currency.
Command Center uses Money Overview / Source Overview.
Values are shown as native buckets and source-provided valuations.
Estimated totals require future explicit network activity settings.
```

Canonical docs:

```text
docs/specs/0014-money-overview-source-taxonomy.md
docs/specs/0013-ledger-assets-valuation.md
docs/specs/0007-first-run-onboarding.md
```

### Net Worth wording

Resolution:

```text
Avoid default single Net Worth hero number in MVP.
Use Money Overview / Source Overview / Your Money Sources.
Single-currency subtotals are allowed when values are compatible.
```

### Money source taxonomy

Resolution:

```text
Money Source = user-recognized platform/institution.
Child container/account = account, card, balance, policy, wallet, portfolio section.
Instrument = thing held or valued.
```

Canonical doc:

```text
docs/specs/0014-money-overview-source-taxonomy.md
```

### Password-protected statement PDFs

Resolution:

```text
Detect locked PDFs.
Prompt user to unlock locally.
Optionally save statement password in OS secret storage.
Store only secret references in SQLite.
Do not send passwords to AI or include them in backups by default.
```

Canonical docs:

```text
docs/specs/0003-gmail-collector.md
docs/specs/0007-first-run-onboarding.md
docs/specs/0009-backup-restore-versioning.md
```

## Audit still needed

- Check future-scope and release assumptions once job engine and testing specs are aligned.
- Check whether future `.agents/` folder should be created after real scripts exist.
- Delete this temp audit after all temporary alignment docs are either removed or replaced by canonical specs.

## Rule for future cleanup

When a temporary alignment decision becomes stable:

```text
1. Update the canonical spec.
2. Update current-state.
3. Update progress-log.
4. Remove or rewrite stale conflicting language.
5. Move the item from this file to resolved.
6. Delete this file when no unresolved audit items remain.
```
