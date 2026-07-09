# AI Agent Reading Order

This file defines the canonical reading path for future AI coding agents.

The goal is to reduce duplicate interpretation, prevent stale-doc conflicts, and let an agent enter implementation mode with the smallest reliable context window.

## Default reading order

Before implementation, read in this order:

```text
1. docs/README.md
2. docs/STRUCTURE.md
3. docs/agent/current-state.md
4. docs/agent/reading-order.md
5. docs/agent/iteration-protocol.md
6. docs/agent/consistency-checklist.md
7. .agents/README.md
8. .agents/ROUTER.md
9. docs/specs/* relevant to the task
10. docs/adr/*.md relevant to the task
11. docs/alignment-temp/* only when active alignment decisions are needed
```

## Source-of-truth hierarchy

When documents conflict, use this order:

```text
1. User's latest explicit instruction in the active conversation
2. docs/agent/current-state.md
3. docs/specs/*.md for implementation-grade detail
4. docs/adr/*.md for architecture decisions
5. docs/alignment-temp/* for temporary unresolved discussion only
6. existing code behavior, only when docs are silent
```

## Canonical specs by topic

```text
Repo structure                docs/specs/0001-repo-structure.md
Database schema               docs/specs/0002-database-schema.md
Gmail collector               docs/specs/0003-gmail-collector.md
Parser contract               docs/specs/0004-parser-contract.md
Review/commit policy          docs/specs/0005-review-and-commit-policy.md
Command Center UI             docs/specs/0006-command-center-ui.md
First-run onboarding          docs/specs/0007-first-run-onboarding.md
Design system                 docs/specs/0008-design-system.md
Backup/restore/versioning     docs/specs/0009-backup-restore-versioning.md
Agentic workflow              docs/specs/0010-agentic-development-workflow.md
Visual tokens                 docs/specs/0011-visual-design-tokens.md
Repo agent workflows          docs/specs/0012-repo-agent-workflows.md
Ledger/assets/valuation       docs/specs/0013-ledger-assets-valuation.md
Money overview/taxonomy       docs/specs/0014-money-overview-source-taxonomy.md
Job engine/error model        docs/specs/0015-job-engine-error-model.md
Testing/fixtures/gates        docs/specs/0016-testing-fixtures-agent-gates.md
```

## Conflict handling protocol

If a conflict is found:

```text
1. Do not implement from the conflicting docs.
2. Check current-state and the newest relevant spec.
3. If the intended answer is clear, update the stale document in the same change.
4. If the answer is unclear, write the question into docs/alignment-temp/grill-backlog.md and ask the user.
5. Record the cleanup in docs/agent/progress-log.md.
```

## Completion gate

A task is not done until:

```text
code behavior matches docs
relevant docs match code behavior
progress log is updated
no known conflicts remain in touched topics
```
