# Development Cycle

Use this workflow for any non-trivial coding or documentation task.

## Loop

1. Run `.agents/scripts/agent-preflight.sh`.
2. Route the prompt with `.agents/ROUTER.md`.
3. Read the selected skill and workflow.
4. For app work, select a slice and generate `.agents/scripts/context-for-slice.sh <slice-id>`.
5. Obey the packet readiness: `STOP` blocks coding; `EVIDENCE ONLY` permits only its named disposable spike/test work; `READY` permits implementation.
6. State assumptions and success criteria.
7. For complex planning, unclear boundaries, document conflicts, redesign, refactoring, performance analysis, or architecture optimization, delegate analysis to the read-only explorer before deciding the implementation approach.
8. For a non-trivial app change, delegate the stable-diff loop within these boundaries:
   - use one implementer as the sole production-code writer;
   - freeze the implementation diff before independent testing and review;
   - let the tester run every test gate declared by the selected slice, `.agents/scripts/agent-preflight.sh`, and `pnpm verify`, reporting exact commands and results without patching production code;
   - require UI evidence for user-visible behavior and `.agents/scripts/harness-self-test.sh` when the change touches `docs/`, `.agents/`, `.codex/`, root `AGENTS.md`, or the docs-harness CI workflow;
   - let the read-only reviewer apply `.agents/workflows/review-code.md` only after that evidence passes;
   - route findings back to the implementer. Any file change invalidates the prior evidence: freeze the new diff, rerun the full applicable set, and re-review the entire cumulative diff.
9. Make the smallest complete change. Trivial changes may stay in the root thread when delegation adds no independent evidence.
10. Verify with the applicable checks above. UI flow inspection is required only for user-visible behavior, not unrelated backend-only changes.
11. Update specs/current-state/progress when meaning changes.
12. Apply the independent semantic gate in `.agents/docs-semantic-review.md` whenever its change-scope trigger matches.
13. Report what changed, what was verified, and what remains.

Never run multiple source-writing agents concurrently. The runtime bindings and pinned models live in `.codex/agents/`; this workflow owns when each role is used.

## Ask Instead Of Guessing

Ask the user when a decision affects:

- money correctness;
- ledger semantics;
- irreversible migrations;
- AI authority;
- privacy or secret handling;
- provider support claims;
- visual identity direction;
- backup/restore compatibility.
