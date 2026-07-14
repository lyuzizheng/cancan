---
name: cancan-implementation-cycle
description: Implement CanCan feature slices with docs, tests, build, UI inspection, and progress updates aligned. Use when building, fixing, or changing CanCan code or implementation specs.
---

# CanCan Implementation Cycle

## Loop

1. Orient with `.agents/scripts/agent-preflight.sh`.
2. Run `.agents/workflows/development-cycle.md` and the task-specific workflow from `.agents/ROUTER.md`.
3. Apply the change-scope trigger in `.agents/docs-semantic-review.md` with a reviewer that did not author the patch. If unavailable, report the gate as blocked.

## Stop And Ask

Ask before deciding money correctness, ledger semantics, irreversible migrations, AI authority, secret handling, provider support, visual identity, or backup compatibility.
