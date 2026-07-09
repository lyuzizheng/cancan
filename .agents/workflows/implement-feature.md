# Implement Feature Workflow

Use this when the user asks to build, add, fix, or change behavior.

## Steps

1. Run `.agents/scripts/agent-preflight.sh`.
2. Read `docs/specs/README.md` and choose the smallest relevant spec set.
3. If no spec owns the behavior, create or update a focused spec first.
4. State assumptions and success criteria.
5. Plan one vertical slice:
   - files/packages touched;
   - schema impact;
   - service/API impact;
   - UI impact;
   - fixture/test strategy;
   - docs updates.
6. Implement the smallest complete slice.
7. Run focused tests/checks.
8. For data changes, reset only the test DB once a DB exists.
9. For parser/AI changes, use deterministic mocked/stored AI outputs.
10. For UI changes, inspect visually once an app exists.
11. Update docs/progress.

## Current Repo State

Application code has not started. Until package scripts exist, implementation tasks may be spec/workflow scaffolding only.
