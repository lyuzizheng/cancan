# Simulated Testing Workflow

Use this to test flows before or alongside automated tests.

## Before App Code Exists

Use simulation as a design exercise:

1. Pick a canonical user scenario from the relevant spec.
2. Walk the expected state transitions.
3. List required fixtures and deterministic assertions.
4. Record gaps in `docs/alignment-temp/grill-backlog.md` or the canonical spec.

## Once App Code Exists

1. Reset the test database.
2. Seed or import deterministic fixtures.
3. Run the flow end-to-end.
4. Capture logs, screenshots, and resulting records.
5. Compare against expected source evidence, staged records, review items, and committed ledger state.
6. File findings or patch the smallest cause.

## Report Template

Use `.agents/templates/simulation-report.md`.
