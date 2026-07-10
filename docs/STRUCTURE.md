# Documentation Structure

## Goal

Keep CanCan's documentation small, canonical, and readable by AI coding agents without duplicate product truth.

## Folder contract

```text
docs/
  README.md
  STRUCTURE.md
  specs/
  adr/
  agent/
  alignment-temp/
```

## What belongs where

### `docs/specs/`

Canonical implementation-grade product and technical specs.

Use specs for:

```text
feature behavior
business logic
data model decisions
UI behavior
security/storage rules
AI/parser contracts
testing/build expectations
```

If something will guide implementation, it belongs in `docs/specs/`.

### `docs/adr/`

Architecture Decision Records.

Use ADRs for architecture decisions with historical context, such as choosing Tauri/React/SQLite or setting the AI ledger boundary.

An ADR's `Status` controls its authority. `Proposed` is not the same as `Accepted`.

ADR files should not duplicate detailed feature specs.

### `docs/agent/`

AI coding agent memory and workflow.

Use it for:

```text
current state
reading order
iteration protocol
consistency checklist
progress log
```

This folder points to specs. It should not become a second product spec layer.

### `docs/alignment-temp/`

Temporary alignment workspace.

Use it for:

```text
active unresolved decision register
focused questions for the next design round
explicit implementation blockers
```

When alignment is complete, stable decisions move into `docs/specs/` and the temp files can be deleted.

### `.agents/`

Repo-local agent operating material.

Use it for:

```text
intent routing
workflow checklists
trigger-oriented skills
deterministic helper scripts
independent semantic review contract
```

The `.agents/` folder describes how future agents work. It must point back to `docs/specs/` for product truth and must not duplicate canonical implementation decisions.

## Removed layer

The old numbered docs layer (`00-product-vision.md` through `11-open-questions.md`) was removed because it duplicated the newer specs and created conflict risk.

Do not recreate broad numbered docs unless there is a very strong reason. Add or update a focused spec instead.

## Source contract

Authority is concern-based, not a misleading total order:

| Concern | Authoritative source |
| --- | --- |
| User direction for the active task | User's latest explicit instruction |
| Intended product and implementation behavior | `docs/specs/*.md` |
| Architecture decision and rationale | `docs/adr/*.md`, according to each ADR's status |
| Current implemented behavior | Code and tests |
| Current phase, focus, and known implementation state | `docs/agent/current-state.md` |
| Unresolved decisions and blockers | `docs/alignment-temp/alignment-progress.md` |
| Agent procedure | `AGENTS.md` and `.agents/` |

Summary, progress, alignment, and harness files may link to canonical behavior but must not restate it as a competing contract.

## Conflict protocol

When sources disagree:

```text
1. Do not silently choose a convenient source.
2. Identify whether the conflict is intended behavior, current behavior, architecture status, or unresolved design.
3. If the user's active instruction resolves it, update every stale projection in the same change.
4. If it needs product, financial, security, privacy, or irreversible data judgment, mark an implementation blocker and ask the user.
5. Run deterministic harness checks and independent semantic review before finishing.
```

## How to add new docs

Before adding a doc:

```text
1. Check specs/README.md for an existing home.
2. Update the existing spec if the topic already exists.
3. Add a new numbered spec only when the topic is genuinely new.
4. Update specs/README.md and agent/current-state.md if the topic changes implementation direction.
5. Record the change in agent/progress-log.md.
6. Run `.agents/scripts/agent-preflight.sh`.
7. Run the independent semantic gate for docs or harness changes.
```

## Anti-duplication rule

A decision should have one canonical home.

Examples:

```text
Gmail collection -> specs/0003-gmail-collector.md
Parser contracts -> specs/0004-parser-contract.md
Review policy -> specs/0005-review-and-commit-policy.md
Command Center UI -> specs/0006-command-center-ui.md
Money overview/taxonomy -> specs/0014-money-overview-source-taxonomy.md
Evidence document UX -> specs/0017-evidence-documents-source-ux.md
```

Other files may link to the canonical spec, but should not restate the full decision.
