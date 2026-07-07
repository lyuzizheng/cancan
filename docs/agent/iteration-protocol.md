# Iteration Protocol

Follow this protocol for every future AI coding session.

## 1. Orient

Read:

```text
docs/README.md
docs/agent/current-state.md
docs/agent/progress-log.md
docs/agent/consistency-checklist.md
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
```

## 3. Check for doc/code impact

Before editing, answer:

```text
Does this change product behavior?
Does this change schemas or storage?
Does this change parser output or validation?
Does this change AI authority or safety boundaries?
Does this change review/commit policy?
Does this change roadmap/progress?
```

If yes, update docs in the same change.

## 4. Implement in small slices

Prefer slices that produce a verifiable result:

```text
schema + migration + tests
service + tests
parser contract + fixture + validation test
UI route + empty/loading/error states
connector scan + mocked fixture
```

Do not jump straight to broad end-to-end automation without a fixture and validation gate.

## 5. Validate

Each slice should have a clear validation artifact:

```text
unit tests
fixture parse snapshot
migration check
UI screenshot/playwright check
manual run log
schema validation output
```

## 6. Update working memory

At the end of a meaningful change:

- update `docs/agent/progress-log.md`;
- update `docs/agent/current-state.md` when focus or decisions change;
- update numbered docs when behavior or architecture changes;
- update ADRs when a major architecture decision is accepted or replaced.

## 7. Keep master coherent

A commit should not leave docs claiming one behavior while code does another. If a feature is partially implemented, mark it partial in progress docs and UI copy.
