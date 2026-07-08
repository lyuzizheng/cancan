---
name: cancan-docs-orientation
description: Orient future agents in CanCan's canonical docs and source-of-truth hierarchy. Use when starting any CanCan task, resolving doc conflicts, or deciding which specs to read.
---

# CanCan Docs Orientation

## Quick Start

1. Run `.agents/scripts/agent-preflight.sh`.
2. Read `docs/README.md`.
3. Read `docs/STRUCTURE.md`.
4. Read `docs/agent/current-state.md`.
5. Read `docs/agent/reading-order.md`.
6. Read only task-relevant specs.

## Conflict Order

```text
1. User's latest explicit instruction
2. docs/agent/current-state.md
3. docs/specs/*.md
4. docs/adr/*.md
5. docs/alignment-temp/* for unresolved work
6. code behavior only when docs are silent
```

## Rule

Do not recreate broad numbered product docs. Add or update focused specs instead.
