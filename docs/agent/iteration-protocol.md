# Iteration Protocol

Follow this protocol for every future AI coding session.

## 1. Orient

Read:

```text
docs/README.md
docs/agent/current-state.md
docs/agent/progress-log.md
docs/agent/consistency-checklist.md
docs/specs/README.md
```

Then read any feature-specific docs relevant to the task.

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

For implementation work, identify the relevant spec in `docs/specs/`. If no spec exists, create or update the spec before coding.

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

For UI work, the agent should run the app, inspect it in browser/computer-use when available, check layout visually, fix issues, and iterate.

For flow work, the agent should reset the database and run integration tests covering the intended path.

## 6. Keep docs/code aligned

At the end of a meaningful change:

- update `docs/agent/progress-log.md`;
- update `docs/agent/current-state.md` when focus or decisions change;
- update numbered docs when behavior or architecture changes;
- update `docs/specs/` when implementation contracts change;
- update ADRs when a major architecture decision is accepted or replaced.

## 7. Ask when blocked

If a decision affects product meaning, financial correctness, AI authority, security, or irreversible data shape and docs do not answer it, ask the user instead of guessing.

## 8. Keep main coherent

A commit should not leave docs claiming one behavior while code does another. If a feature is partially implemented, mark it partial in progress docs and UI copy.
