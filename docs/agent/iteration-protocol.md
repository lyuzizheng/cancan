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

Plan implementation with `.agents/workflows/implement-feature.md` (Stage 2 of the issue loop): select a slice from `docs/agent/implementation-slices.md`, generate `.agents/scripts/context-for-slice.sh <slice-id>`, and obey the generated `STOP` / `EVIDENCE ONLY` / `READY` readiness. The plan checklist (files/packages, schema, service/API, UI, tests, build/run validation, docs) is canonical in `issue-delivery.md` Stage 1. If no slice/spec owns the behavior, update the focused plan/spec before coding.

## 4. Implement in small slices

Size execution by consequence using the canonical risk tiers in `.agents/workflows/development-cycle.md`; that workflow owns the Fast/Standard/High-risk definitions and the tier-sized verification order. Never run multiple source-writing agents concurrently.

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
