# Iteration Protocol

Follow this protocol for every future AI coding session.

## 1. Orient

Run `.agents/scripts/agent-preflight.sh`, follow `docs/agent/reading-order.md`, then read task-relevant canonical specs and ADRs.

## 2. Classify the task

Classify the task as one or more of:

```text
product-docs
architecture-docs
schema/data-model
frontend-ui
local-backend/service
parser/ai
plugin/connector
security/backup
tests/fixtures
build/package
```

## 3. Plan from specs

For implementation work, select a slice from `docs/agent/implementation-slices.md` and generate `.agents/scripts/context-for-slice.sh <slice-id>`. If no slice/spec owns the behavior, update the focused plan/spec before coding.

Obey the generated readiness result. `STOP` blocks coding. `EVIDENCE ONLY` permits only the bounded disposable spike/test work named by the slice and never production code. `READY` permits implementation.

A coding plan should include:

```text
files/packages touched
schema/migration impact
service/API impact
UI impact
test strategy
build/run validation
required doc updates
```

## 4. Implement in small slices

Size execution by consequence using `.agents/workflows/development-cycle.md`. Fast PR-comment maintenance stays in the root thread with focused checks and PR CI. Standard work adds at most one independent role when it provides material evidence. High-risk app changes use one production-code writer, applicable independent code review, and one final full relevant app gate after that review passes. Docs/harness changes run preflight and harness self-test before semantic review. After a finding, rerun affected focused checks and re-review the entire cumulative diff; do not repeat unrelated full builds. Never run multiple source-writing agents concurrently.

Prefer slices that produce a verifiable result:

```text
schema + migration + tests
service + tests
parser contract + fixture + validation test
UI route + empty/loading/error states
connector scan + mocked fixture
build/package check
```

Do not jump straight to broad end-to-end automation without fixtures and validation gates.

## 5. Validate like an autonomous coding agent

Each slice should have a clear validation artifact:

```text
unit tests
integration tests with database reset
fixture parse snapshot
migration check
query benchmark for hot paths
UI screenshot/browser check
manual run log
schema validation output
build/package output
```

Every implementation, simulated-testing, or code-review role used for a task must share the same slice ID and generated context packet.

For UI work, the agent should run the app, inspect it in browser/computer-use when available, check layout visually, fix issues, and iterate.

For flow work, the agent should reset the database and run integration tests covering the intended path.

Use the narrowest layer that proves the change. Add Tauri and UI flow evidence only when the slice crosses those boundaries; user-visible flows must not stop at a mocked component.

## 6. Keep docs/code aligned

At the end of a meaningful change:

- update `docs/agent/progress-log.md`;
- update `docs/agent/current-state.md` when focus or decisions change;
- update `docs/specs/` when implementation contracts change;
- update ADRs when a major architecture decision is accepted or replaced.
- run `.agents/scripts/agent-preflight.sh`;
- run independent semantic review when docs, harness, or project agent configuration files changed.

## 7. Ask when blocked

If a decision affects product meaning, financial correctness, AI authority, security, or irreversible data shape and docs do not answer it, ask the user instead of guessing.

## 8. Keep main coherent

A commit should not leave docs claiming one behavior while code does another. If a feature is partially implemented, record current behavior in `current-state.md`; if a decision is unresolved, keep it in the active alignment register and add an implementation blocker to the affected spec.
