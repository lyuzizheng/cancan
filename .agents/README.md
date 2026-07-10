# CanCan Agent Harness

This folder contains procedure only. Intended product and implementation behavior belongs in `docs/specs/`; architecture decisions belong in status-bearing ADRs.

## Start

1. Run `.agents/scripts/agent-preflight.sh`.
2. Follow `docs/agent/reading-order.md`.
3. Select the task-relevant skill and workflow from `.agents/ROUTER.md`.
4. Read only the relevant canonical specs and ADRs.

## Shape

```text
.agents/
  README.md
  ROUTER.md
  docs-semantic-review.md
  workflows/   detailed task loops
  skills/      trigger-oriented entry points
  scripts/     deterministic gates and generators
```

Roles, repeated product rules, static priority lists, generic templates, and placeholder plugin guidance are intentionally excluded. They created additional sources of truth without adding executable guarantees.

## Two-layer gate

Every meaningful change uses both layers:

```text
deterministic gate
  -> paths, spec index, duplicate numbers, skill metadata, shell syntax,
     sensitive fixture tracking, stale harness references, whitespace

semantic gate
  -> an independent agent reviews changed docs and harness files for contradiction,
     hidden product decisions, stale status, unsafe implementation guidance,
     and duplicate ownership
```

Run the deterministic gate with `.agents/scripts/agent-preflight.sh`. Test the gate itself with `.agents/scripts/harness-self-test.sh`.

For docs or harness changes, prepare the semantic review with `.agents/scripts/docs-review-packet.sh HEAD` and follow `.agents/docs-semantic-review.md`. A semantic reviewer may identify a decision that needs user input, but must not decide it.

Read implementation state from `docs/agent/current-state.md`; do not copy it into the harness. App commands may be added only when the referenced scripts and paths are real.
