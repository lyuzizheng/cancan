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
6. Select the smallest consequence-based execution tier and only the roles justified by 0012-repo-agent-workflows.md and .agents/workflows/development-cycle.md
7. Let one production-code writer complete the smallest READY-slice change, focused tests, and required docs/progress updates
8. Freeze the cumulative diff and generate the shared implementation review packet when the selected tier requires review
9. Route findings back to the writer, rerun affected focused checks, and re-review the cumulative diff as required by 0012
10. Run the selected tier's final relevant gate once after required code review passes; docs/harness changes follow their deterministic-then-semantic order
11. Ask the user whenever product, security, or data decisions remain unresolved
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
- Every implementation, testing, or review role used for a task shares one machine-checked slice context rather than loading all specs or choosing different contracts.
- One writer owns production-code changes; independent testing and review are applied only when required by the consequence-based path in `0012-repo-agent-workflows.md`.
- Documentation/harness validation follows `0012-repo-agent-workflows.md`.
- UI changes cannot be completed without visual inspection.
- Integration flows require database reset coverage.
- Docs remain part of the implementation contract.
