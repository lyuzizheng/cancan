# Development Cycle

Use this workflow for any non-trivial coding or documentation task.

## Loop

0. Ensure a tracked GitHub issue exists for this work (search first; if none, file one with the `cancan-issue-triage` skill — policy in the `issue-delivery` workflow). Implementation never starts issue-less.
1. Run `.agents/scripts/agent-preflight.sh`.
2. Route the prompt with `.agents/ROUTER.md`.
3. Read the selected skill and workflow.
4. For app work, select a slice and generate `.agents/scripts/context-for-slice.sh <slice-id>`.
5. Obey the packet readiness: `STOP` blocks coding; `EVIDENCE ONLY` permits only its named disposable spike/test work; `READY` permits implementation.
6. State assumptions, success criteria, and the smallest justified execution tier:
   - **Fast:** localized PR-comment fixes or mechanical maintenance with no contract, migration, security, financial, or harness-authority change. Keep work in the root thread; run the focused regression test, the affected package/type check, and `git diff --check`, then let PR CI provide broader repository coverage.
   - **Standard:** a localized behavior change with bounded consequences. Keep one production-code writer and focused tests. Add at most one independent tester or reviewer only when it supplies evidence the root agent or CI cannot; do not use both by default.
   - **High risk:** financial/data correctness, ledger or auto-commit behavior, migrations or irreversible data, security/privacy/secrets, release/update, or agent-harness authority. Keep one production-code writer, require the applicable independent review or semantic gate, and run the full relevant local gate once on the final stable diff. Use a separate tester only when the user requests it or execution independence materially changes the evidence.
7. Risk follows consequences, not line count. The user may explicitly raise a tier. Do not raise it merely because a custom agent exists.
8. Use the read-only explorer only when complex planning, unclear boundaries, document conflicts, redesign, refactoring, performance analysis, or architecture optimization makes separate exploration useful.
   When an independent role is justified, hand it the exact task, slice ID, review base, selected execution tier and justification, assumptions, success criteria, canonical sources inspected at the exact head commit, verification evidence, and prior findings. Do not duplicate the root transcript or tool history; the role reads every indexed canonical source in full and reads the cumulative diff directly from the shared repository.
9. Make the smallest complete change. Never run multiple source-writing agents concurrently.
10. Verify in this order:
   - while editing, run only focused checks that can guide the next change;
   - when independent code review is required, freeze the cumulative diff after focused checks and review it before the expensive final app gate;
   - after review findings, rerun affected focused checks and re-review the entire cumulative diff, but do not repeat unrelated full builds;
   - after the required code review passes, run the tier's final applicable app gate once. High-risk app code may require selected-slice gates and `pnpm verify`;
   - for docs/harness-only changes, run preflight and harness self-test before the required semantic review, and do not run an unrelated app build;
   - if that final gate causes a code fix, re-review the changed cumulative diff and rerun the failed/final gate.
   UI flow inspection is required only for user-visible behavior.
   For pull-request work, keep the PR draft while review findings are collected, request available review lanes together, batch the resulting fixes into one writer pass, and mark the PR ready only for the final CI evidence. Do not push once per review comment.
11. Update specs/current-state/progress when meaning changes.
12. Apply the independent semantic gate in `.agents/docs-semantic-review.md` whenever its change-scope trigger matches.
13. Report what changed, what was verified, and what remains.

The runtime bindings and pinned models live in `.codex/agents/`; this workflow owns when each role is used.

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
