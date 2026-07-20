# Implement Feature Workflow

Use this when the user asks to build, add, fix, or change behavior.

## Steps

1. Run `.agents/scripts/agent-preflight.sh`.
2. Select one ID from `docs/agent/implementation-slices.md`.
3. Generate `.agents/scripts/context-for-slice.sh <slice-id>`. Open the full contents of every source listed by the generated index from the exact working tree and head before planning or coding; record that head and source list for review. Use this bounded spec/ADR set instead of reading unrelated specs.
4. Obey the packet's readiness result:
   - `STOP`: do not code the slice;
   - `EVIDENCE ONLY`: run only the bounded disposable spike/test work named by the slice; do not add production code;
   - `READY`: implementation may proceed.
5. If no slice owns the behavior, update the implementation-slice plan before coding.
6. State assumptions and success criteria.
7. Plan the smallest complete part of that vertical slice:
   - files/packages touched;
   - schema impact;
   - service/API impact;
   - UI impact;
   - fixture/test strategy;
   - docs updates.
8. Implement it.
   - When delegated, the implementer is the only production-code writer.
   - The implementer owns focused tests for the changed behavior and failure paths.
9. Use the risk-sized verification order in `.agents/workflows/development-cycle.md`; run focused checks while iterating and the applicable final app gate once after required code review passes.
10. For data changes, reset only the test DB once a DB exists.
11. For parser/AI changes, use deterministic mocked/stored AI outputs.
12. For UI changes, inspect visually once an app exists.
13. Update docs/progress and generate `.agents/scripts/implementation-review-packet.sh <slice-id> <base>` for review.

Freeze the cumulative diff before any required independent review. Route findings back to the production-code writer instead of allowing concurrent fixes; do not add tester/reviewer roles that the selected execution tier does not justify.

Read current implementation state directly from the indexed `docs/agent/current-state.md`; the compact packet does not contain or replace that source. Do not copy state or planned command names into this workflow.
