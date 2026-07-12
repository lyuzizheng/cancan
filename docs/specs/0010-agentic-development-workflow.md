# 0010. Agentic Development Workflow Spec

## Goal

Define how a solo developer and AI coding agent should develop CanCan autonomously while keeping product docs, implementation, tests, build, and UI quality aligned.

## Required agent loop

For each feature slice:

```text
1. Select a machine-checked ID from docs/agent/implementation-slices.md
2. Generate .agents/scripts/context-for-slice.sh <slice-id>
3. Obey the packet's STOP, EVIDENCE ONLY, or READY boundary
4. For EVIDENCE ONLY, run only the named disposable spike/test work and add no production code
5. Plan files, tests, build, and docs impact from the packet
6. Implement the smallest complete part of a READY slice
7. Reset test database if data layer is involved
8. Run the slice's unit/integration/fixture/visual gates with real commands
9. Run benchmark/checks if hot SQL paths are touched
10. Build/package app when relevant
11. Inspect UI with browser/computer-use/Chrome MCP when UI changed
12. Fix issues and repeat
13. Update docs/progress
14. Generate the shared implementation review packet
15. Run the repo harness gates required by 0012
16. Ask user when product/security/data decisions are unclear
```

## Browser/computer-use policy

UI features must be inspected visually with the best available tool in the current environment.

Acceptable tools may include:

- browser automation;
- computer-use;
- Chrome MCP;
- Playwright screenshots;
- installed repo-local skills/workflows.

The repo-local harness is defined in `0012-repo-agent-workflows.md`. UI tooling remains flexible, but the visual-inspection outcome is required.

## Done criteria

A feature is not done until:

- tests pass;
- database reset path works if applicable;
- migrations apply cleanly if applicable;
- build succeeds if app code changed;
- UI was visually inspected if UI changed;
- docs/specs/progress are updated;
- unresolved decisions are recorded or asked.
- the repo harness gates required by `0012-repo-agent-workflows.md` pass.

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
- Implementation, testing, and review use one machine-checked slice context rather than loading all specs or choosing different contracts.
- Documentation/harness validation follows `0012-repo-agent-workflows.md`.
- UI changes cannot be completed without visual inspection.
- Integration flows require database reset coverage.
- Docs remain part of the implementation contract.
