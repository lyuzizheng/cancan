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

1. Select the same implementation slice ID used by the feature.
2. Generate `.agents/scripts/context-for-slice.sh <slice-id>`, open the full contents of every source listed by the generated index from the exact working tree and head, record that head and source list, and obey its `STOP`, `EVIDENCE ONLY`, or `READY` boundary.
3. Reset only the test database.
4. Seed or import deterministic fixtures.
5. Use mocked/stored AI outputs for parser tests.
6. Run the flow end-to-end.
7. Capture logs, screenshots, and resulting records.
8. Compare against expected source evidence, staged records, review items, jobs, and committed ledger state.
9. Report reproducible findings to the root agent. Do not patch production code in the tester role.

Scenario requirements are owned by `docs/specs/0016-testing-fixtures-agent-gates.md`. Report the scenario, expected state transitions, actual result, gaps, and follow-up tests.

Use the narrowest layer that proves the changed behavior. Add Tauri-boundary or UI-flow inspection only when the slice crosses those boundaries; do not turn backend-only changes into artificial UI tests.
