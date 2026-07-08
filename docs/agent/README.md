# AI Agent Operating Manual

This folder exists so future AI coding agents can keep product docs, implementation progress, and code behavior aligned.

Before making code changes, an agent must read:

1. `docs/README.md`
2. `docs/agent/current-state.md`
3. `docs/agent/reading-order.md`
4. `docs/agent/iteration-protocol.md`
5. `docs/agent/consistency-checklist.md`
6. The feature-specific specs and product/architecture docs touched by the task

## Source-of-truth hierarchy

Use this order when resolving conflicts:

1. User's newest instruction in the active conversation.
2. `docs/agent/current-state.md` for current implementation focus.
3. `docs/specs/*.md` for implementation-grade detail.
4. ADRs in `docs/adr/` for accepted architecture decisions.
5. Product/architecture docs in `docs/00-*.md` through `docs/11-*.md`.
6. Existing code behavior, only when docs are silent.

If code and docs disagree, do not silently choose one. Update docs and code together or record the discrepancy in `docs/agent/progress-log.md`.

## What belongs here

```text
current-state.md          current decisions and immediate focus
reading-order.md          canonical read path and conflict resolution order
progress-log.md           dated progress entries and next actions
iteration-protocol.md     how AI agents should plan, implement, validate, and update docs
consistency-checklist.md  checks before a task is considered complete
```

Do not put large design essays here. Put product design in numbered docs/specs and use this folder as the working memory layer.
