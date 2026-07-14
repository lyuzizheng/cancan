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
6. Use the read-only explorer for complex planning, unclear boundaries, document conflicts, redesign, refactoring, performance analysis, or architecture optimization
7. Let one implementer write the smallest complete part of a READY slice and its focused tests
8. Freeze the implementation diff before independent testing and review
9. Let the tester run the strongest relevant deterministic, database, Tauri, and UI gates without patching production code
10. Let an independent read-only reviewer judge the stable diff and evidence
11. Route findings back to the implementer, then repeat testing and review
12. Reset test database if data layer is involved
13. Run benchmark/checks if hot SQL paths are touched
14. Build/package app when relevant
15. Inspect UI with browser/computer-use/Chrome MCP when user-visible behavior changed
16. Update docs/progress and generate the shared implementation review packet
17. Run the repo harness gates required by 0012
18. Ask user when product/security/data decisions are unclear
```

Trivial changes may stay in the root thread when delegation would add no independent evidence. Multiple source-writing agents must not run concurrently.

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
- One implementer owns production-code writes; testing and review remain independent from implementation.
- Documentation/harness validation follows `0012-repo-agent-workflows.md`.
- UI changes cannot be completed without visual inspection.
- Integration flows require database reset coverage.
- Docs remain part of the implementation contract.
