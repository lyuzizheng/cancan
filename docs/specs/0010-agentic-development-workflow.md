# 0010. Agentic Development Workflow Spec

## Goal

Define how a solo developer and AI coding agent should develop CanCan autonomously while keeping product docs, implementation, tests, build, and UI quality aligned.

## Required agent loop

For each feature slice:

```text
1. Read docs/agent/current-state.md
2. Read relevant docs/specs/*.md
3. Plan files, tests, build, and docs impact
4. Implement the smallest complete slice
5. Reset test database if data layer is involved
6. Run unit and integration tests
7. Run benchmark/checks if hot SQL paths are touched
8. Build/package app when relevant
9. Inspect UI with browser/computer-use/Chrome MCP when UI changed
10. Fix issues and repeat
11. Update docs/progress
12. Ask user when product/security/data decisions are unclear
```

## Browser/computer-use policy

UI features must be inspected visually with the best available tool in the current environment.

Acceptable tools may include:

- browser automation;
- computer-use;
- Chrome MCP;
- Playwright screenshots;
- future installed repo-local skills/workflows.

The repo may include `.agents/` workflows or skills later to pin these procedures. If added, they become part of the agent operating contract.

## Done criteria

A feature is not done until:

- tests pass;
- database reset path works if applicable;
- migrations apply cleanly if applicable;
- build succeeds if app code changed;
- UI was visually inspected if UI changed;
- docs/specs/progress are updated;
- unresolved decisions are recorded or asked.

## Integration test expectations

Important flows should have integration tests that can run from a clean database:

```text
create vault-like test context
create money source/account
import evidence fixture
parse fixture into external records
validate records
create review items
commit accepted records
query Command Center summary
```

LLM-dependent tests should use deterministic mocked model outputs or stored parse fixtures.

## When to ask the user

Ask instead of guessing when the decision affects:

- money correctness;
- ledger semantics;
- irreversible migrations;
- AI authority;
- privacy/secret handling;
- provider support claim;
- visual identity direction;
- backup/restore compatibility.

## Acceptance criteria

- Agents have a repeatable loop for autonomous development.
- UI changes cannot be completed without visual inspection.
- Integration flows require database reset coverage.
- Docs remain part of the implementation contract.
