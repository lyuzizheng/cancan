# Temporary Doc Consistency Audit

Status: active temporary audit.

Purpose: track duplicate/conflicting documentation while the product spec is still being aligned. Delete this file when the permanent docs are clean and stable.

## Current resolved conflicts

### Base currency

Resolution:

```text
MVP does not ask for base currency.
Command Center uses Money Overview / Source Overview.
Values are shown as native buckets and source-provided valuations.
Estimated totals require future explicit network activity settings.
```

Permanent docs:

```text
docs/specs/0014-money-overview-source-taxonomy.md
docs/specs/0013-ledger-assets-valuation.md
docs/02-domain-model.md
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

### Password-protected statement PDFs

Resolution:

```text
Detect locked PDFs.
Prompt user to unlock locally.
Optionally save statement password in OS secret storage.
Store only secret references in SQLite.
Do not send passwords to AI or include them in backups by default.
```

Permanent docs:

```text
docs/specs/0003-gmail-collector.md
docs/specs/0007-first-run-onboarding.md
docs/06-local-storage-security-backup.md
```

## Audit still needed

- Check numbered docs for stale `base currency`, `net worth`, or `freshness score` wording.
- Check roadmap for phase order drift after Gmail/AI/parser decisions.
- Check `docs/specs/0002-database-schema.md` after real SQL migrations exist.
- Check whether crypto/Bitget remains future-scope only or becomes an explicit supported source.
- Check whether `.agents/` should be created after scripts exist.

## Rule for future cleanup

When a temporary alignment decision becomes stable:

```text
1. Update the canonical spec.
2. Update current-state.
3. Update progress-log.
4. Remove or rewrite stale conflicting language in older docs.
5. Move the item from this file to resolved, then delete this file when all items are resolved.
```
