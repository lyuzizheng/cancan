# CanCan Canonical Specs

`docs/specs/` is the canonical implementation source of truth for CanCan.

Future AI coding agents should implement from these specs, not from removed historical numbered docs.

## Reading rule

Read only the specs relevant to the task plus `docs/agent/current-state.md` and `docs/agent/reading-order.md`.

Do not load every spec by default unless the task spans the whole system.

## Spec map

| Spec | Canonical topic |
| --- | --- |
| `0001-repo-structure.md` | Monorepo/package layout and code boundaries |
| `0002-database-schema.md` | SQLite/SQLCipher schema policy, indexes, benchmark, DB reset |
| `0003-gmail-collector.md` | Gmail OAuth/API, rules, sync, attachments, protected PDF passwords |
| `0004-parser-contract.md` | Native extraction + OCR + AI normalization contracts |
| `0005-review-and-commit-policy.md` | Review, deterministic validation, auto-commit policy |
| `0006-command-center-ui.md` | Main app shell, dashboard modules, review surface, AI Assistant entry |
| `0007-first-run-onboarding.md` | First launch, vault setup, provider setup, startup sequence |
| `0008-design-system.md` | Visual direction and design principles |
| `0009-backup-restore-versioning.md` | Backup bundles, restore, schema/version compatibility, secrets policy |
| `0010-agentic-development-workflow.md` | Build/test/package/inspect workflow for AI coding agents |
| `0011-visual-design-tokens.md` | Colors, typography, spacing, radius, motion tokens |
| `0012-repo-agent-workflows.md` | Repo-local agent harness, deterministic gates, and semantic review |
| `0013-ledger-assets-valuation.md` | Ledger, valuations, snapshots, trades, multi-currency behavior |
| `0014-money-overview-source-taxonomy.md` | Money Overview, no base currency, source/account/instrument taxonomy |
| `0015-job-engine-error-model.md` | Durable jobs, blocked states, retry, recovery, errors |
| `0016-testing-fixtures-agent-gates.md` | Fixture privacy, deterministic tests, DB reset, future application gates |
| `0017-evidence-documents-source-ux.md` | Source Documents placement, document detail, states, search, and actions |
| `0018-app-updates-open-source-release.md` | App updates, forward compatibility, parser delivery, and open-source GitHub releases |

## Adding a new spec

Add a new spec only when no existing spec can own the decision.

Required format:

```text
# 00XX. Title

## Goal
## Stable decisions
## Data/API/UI behavior when relevant
## Edge cases
## Tests / acceptance criteria
```

## Updating specs

When changing a decision:

```text
1. Update the canonical spec.
2. Update docs/agent/current-state.md if current implementation direction changes.
3. Update docs/agent/progress-log.md.
4. Remove or rewrite conflicting temp notes.
```
