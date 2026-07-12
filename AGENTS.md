# CanCan Agent Entry Point

This file is the tool-neutral entry point for coding agents working in this repository.

1. Run `.agents/scripts/agent-preflight.sh`.
2. Follow `docs/agent/reading-order.md` and the source contract in `docs/STRUCTURE.md`.
3. Select the task-relevant skill and workflow from `.agents/ROUTER.md`.
4. For app implementation, testing, or review, select a slice from `docs/agent/implementation-slices.md` and generate its shared context with `.agents/scripts/context-for-slice.sh <slice-id>`.
5. Do not implement through a slice's active blockers or infer unresolved product, financial, security, or irreversible data decisions.
6. If any docs or harness files change, run an independent semantic review using `.agents/docs-semantic-review.md` before finishing.

`docs/specs/` owns intended product and implementation behavior. `.agents/` owns procedure only.
