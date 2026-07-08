# CanCan Agent Workspace

This folder is the repo-local operating guide for future coding agents.

It does not replace `docs/specs/`. Canonical product and implementation truth stays in `docs/specs/`; this folder explains how an agent should work with that truth.

## Start Here

1. Run `.agents/scripts/agent-preflight.sh`.
2. Read `docs/README.md`.
3. Read `docs/STRUCTURE.md`.
4. Read `docs/agent/current-state.md`.
5. Read `docs/agent/reading-order.md`.
6. Read `.agents/ROUTER.md`.
7. Pick the role/workflow that matches the user's prompt.
8. Read only task-relevant specs and ADRs.

## Folder Map

```text
.agents/
  README.md
  ROUTER.md
  rules/       always-on project rules for agent behavior
  workflows/   repeatable task loops
  roles/       role cards selected by user prompt
  skills/      repo-local skill instructions
  plugins/     guidance for connectors/plugins/capabilities
  scripts/     deterministic checks and helper generators
  templates/   reusable report/spec/checklist templates
```

## Core Rule

Every agent cycle follows this shape:

```text
orient -> route role -> read canonical docs -> plan -> execute smallest slice -> verify -> update docs -> report
```

Do not invent product truth in `.agents/`. If a decision affects implementation, put it in `docs/specs/` or an ADR.

## Current Limits

Application code has not started. Workflows that mention tests, DB reset, package, UI inspection, or simulated flows define the required shape, but must be wired to real commands once app code exists.
