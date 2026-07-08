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

Use ADRs for stable decisions with historical context, such as choosing Tauri/React/SQLite or setting the AI ledger boundary.

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
grill-me question backlog
temporary decision tracking
doc consistency audits while the design is still moving
```

When alignment is complete, stable decisions move into `docs/specs/` and the temp files can be deleted.

### `.agents/`

Repo-local agent operating material.

Use it for:

```text
role routing
workflow checklists
agent-facing skills
deterministic helper scripts
report/spec templates
```

The `.agents/` folder describes how future agents work. It must point back to `docs/specs/` for product truth and must not duplicate canonical implementation decisions.

## Removed layer

The old numbered docs layer (`00-product-vision.md` through `11-open-questions.md`) was removed because it duplicated the newer specs and created conflict risk.

Do not recreate broad numbered docs unless there is a very strong reason. Add or update a focused spec instead.

## Source-of-truth order

When content conflicts:

```text
1. User's latest explicit instruction in the active conversation
2. docs/agent/current-state.md
3. docs/specs/*.md
4. docs/adr/*.md
5. docs/alignment-temp/* for unresolved temporary work
6. code behavior only when docs are silent
```

## How to add new docs

Before adding a doc:

```text
1. Check specs/README.md for an existing home.
2. Update the existing spec if the topic already exists.
3. Add a new numbered spec only when the topic is genuinely new.
4. Update specs/README.md and agent/current-state.md if the topic changes implementation direction.
5. Record the change in agent/progress-log.md.
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
```

Other files may link to the canonical spec, but should not restate the full decision.
