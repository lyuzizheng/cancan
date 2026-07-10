# Simulated Testing Workflow

Use this to test flows before or alongside automated tests.

Canonical fixture/testing policy: `docs/specs/0016-testing-fixtures-agent-gates.md`.

## Before App Code Exists

Use simulation as a design exercise:

1. Pick a canonical user scenario from the relevant spec.
2. Walk the expected state transitions.
3. List required fixtures and deterministic assertions.
4. Decide which fixture layer is appropriate: private, redacted, or synthetic.
5. Record unresolved design gaps in `docs/alignment-temp/alignment-progress.md`; record accepted behavior in the canonical spec.

Fixture privacy and live-service boundaries are owned by `docs/specs/0016-testing-fixtures-agent-gates.md`.

## Once App Code Exists

1. Reset only the test database.
2. Seed or import deterministic fixtures.
3. Use mocked/stored AI outputs for parser tests.
4. Run the flow end-to-end.
5. Capture logs, screenshots, and resulting records.
6. Compare against expected source evidence, staged records, review items, jobs, and committed ledger state.
7. File findings or patch the smallest cause.

Scenario requirements are owned by `docs/specs/0016-testing-fixtures-agent-gates.md`. Report the scenario, expected state transitions, actual result, gaps, and follow-up tests.
