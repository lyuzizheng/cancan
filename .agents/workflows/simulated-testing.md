# Simulated Testing Workflow

Use this to test flows before or alongside automated tests.

Canonical fixture/testing policy: `docs/specs/0016-testing-fixtures-agent-gates.md`.

## Before App Code Exists

Use simulation as a design exercise:

1. Pick a canonical user scenario from the relevant spec.
2. Walk the expected state transitions.
3. List required fixtures and deterministic assertions.
4. Decide which fixture layer is appropriate: private, redacted, or synthetic.
5. Record gaps in `docs/alignment-temp/grill-backlog.md` or the canonical spec.

## Fixture Selection

```text
fixtures-private/     local real samples, ignored by Git
fixtures-redacted/    manually redacted, commit only after review
fixtures-synthetic/   generated safe samples, preferred for CI
```

Never use live LLM, live Gmail, or real external finance accounts as default test gates.

## Once App Code Exists

1. Reset only the test database.
2. Seed or import deterministic fixtures.
3. Use mocked/stored AI outputs for parser tests.
4. Run the flow end-to-end.
5. Capture logs, screenshots, and resulting records.
6. Compare against expected source evidence, staged records, review items, jobs, and committed ledger state.
7. File findings or patch the smallest cause.

## Required Scenario Types

For supported provider document types, cover:

```text
normal statement
edge-case statement
duplicate/reconciliation scenario
password-protected PDF when applicable
```

## Report Template

Use `.agents/templates/simulation-report.md`.
