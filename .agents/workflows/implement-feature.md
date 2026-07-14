# Implement Feature Workflow

Use this when the user asks to build, add, fix, or change behavior.

## Steps

1. Run `.agents/scripts/agent-preflight.sh`.
2. Select one ID from `docs/agent/implementation-slices.md`.
3. Generate `.agents/scripts/context-for-slice.sh <slice-id>`; use its spec/ADR set instead of reading every spec.
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
9. Run the manifest's test gates using real commands that already exist.
10. For data changes, reset only the test DB once a DB exists.
11. For parser/AI changes, use deterministic mocked/stored AI outputs.
12. For UI changes, inspect visually once an app exists.
13. Update docs/progress and generate `.agents/scripts/implementation-review-packet.sh <slice-id> <base>` for review.

Freeze the diff before handing it to the independent tester and reviewer. Route their findings back to the implementer instead of allowing concurrent fixes.

Read current implementation state from the generated packet. Do not copy state or planned command names into this workflow.
