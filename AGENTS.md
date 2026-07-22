# CanCan Agent Entry Point

This file is the tool-neutral entry point for coding agents working in this repository.

1. Run `.agents/scripts/agent-preflight.sh`.
2. Follow `docs/agent/reading-order.md` and the source contract in `docs/STRUCTURE.md`.
3. Select the task-relevant skill and workflow from `.agents/ROUTER.md`.
4. For app implementation, testing, or review, select a slice from `docs/agent/implementation-slices.md`, generate its shared index with `.agents/scripts/context-for-slice.sh <slice-id>`, then open the full contents of every listed source from the exact working tree and head before acting.
5. Size the workflow by consequence using `.agents/workflows/development-cycle.md`. Simple PR-comment maintenance stays in the root thread with focused checks. Add an independent role only when it supplies material evidence; reserve the full high-risk path for financial/data correctness, migrations, security/privacy/secrets, auto-commit, release/update, or agent-harness authority changes. Never run multiple source-writing agents concurrently.
6. Do not implement through a slice's active blockers or infer unresolved product, financial, security, or irreversible data decisions.
7. If any docs, harness, or project agent configuration files change, run an independent semantic review using `.agents/docs-semantic-review.md` before finishing.
8. Jira linkage is not required in this repository. Do not use the `gh-create-pr-from-branch` skill; use conventional branch names such as `feat/...`, `fix/...`, or `chore/...`, Conventional Commit subjects, and concise conventional pull-request titles and bodies.
9. Kimi Code CLI leads frontend craft. Frontend code means the `apps/desktop` renderer and `packages/ui`. Other agents may implement frontend code, but UX direction, advanced visual design, and the final designer-level review of customer-facing UI governed by the visual design specs (`docs/specs/0006-command-center-ui.md`, `docs/specs/0008-design-system.md`, `docs/specs/0011-visual-design-tokens.md`, `docs/specs/0017-evidence-documents-source-ux.md`) are reserved to Kimi Code CLI: when a task needs that depth, route it through Kimi Code CLI.

`docs/specs/` owns intended product and implementation behavior. `.agents/` owns procedure only. `.codex/agents/` contains only executable role, model, and subagent permission defaults; it does not set the main agent's permission mode or guarantee a child's effective runtime sandbox.
