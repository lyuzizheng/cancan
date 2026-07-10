# AI Agent Operating Manual

This folder exists so future AI coding agents can keep product docs, implementation progress, and code behavior aligned.

Before making changes, run `.agents/scripts/agent-preflight.sh` and follow [`reading-order.md`](./reading-order.md).

## Source contract

The source contract lives only in [`docs/STRUCTURE.md`](../STRUCTURE.md). This folder records phase, progress, and procedure; it does not override product behavior.

## Relationship with `.agents/`

```text
docs/agent/   persistent project memory and current state
.agents/      repo-local skills, workflows, deterministic gates, and semantic review
```

Neither folder owns product truth. Product and implementation truth belongs in `docs/specs/` or ADRs.

## What belongs here

```text
current-state.md          phase, focus, and current implementation state
reading-order.md          canonical read path and conflict resolution order
progress-log.md           dated progress entries and next actions
iteration-protocol.md     how AI agents should plan, implement, validate, and update docs
consistency-checklist.md  checks before a task is considered complete
```

Do not put large design essays here. Put product design in `docs/specs/` and use this folder as the working memory layer.
