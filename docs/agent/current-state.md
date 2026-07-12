# Current State

Last updated: 2026-07-13

## Product phase

CanCan is in documentation/design alignment. Application code has not started.

The current product target is a Gmail-first local financial evidence vault and polished personal finance/account-record workspace with AI-assisted parsing, deterministic validation, policy-gated ledger commit, and source-backed money views.

## Implementation state

- The repository contains documentation and the coding-agent harness only.
- No app packages, migrations, runtime commands, build scripts, or UI implementation exist yet.
- App-specific test, DB reset, package, and visual-inspection commands must be added only after their real implementations exist.

## Documentation state

- `docs/specs/README.md` is the only maintained canonical spec index.
- `docs/STRUCTURE.md` is the only source contract and conflict protocol.
- ADR authority follows each ADR's explicit status; ADR 0001 remains `Proposed` and ADR 0002 is `Accepted`.
- `docs/alignment-temp/alignment-progress.md` is the only active unresolved decision register.
- `docs/specs/0018-app-updates-open-source-release.md` owns the accepted open-source GitHub release/update direction and its remaining blockers.
- `.agents/scripts/agent-preflight.sh` provides deterministic gates.
- `.agents/docs-semantic-review.md` defines the independent semantic gate for docs and harness changes.

The old broad numbered product-doc layer and duplicated alignment/harness projections have been removed. Do not recreate them.

## Immediate next design tasks

Use the order in `docs/alignment-temp/alignment-progress.md`; do not copy that order into this summary.

Do not implement an area whose canonical spec carries an unresolved implementation blocker.

## Current scope guardrail

Do not start API trading, payments, bill pay, mobile sync, full budgeting, tax reporting, autonomous financial advice, or external market-data/FX fetching before the core evidence/reconciliation loop works and the relevant canonical spec changes.
