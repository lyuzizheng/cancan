# Alignment Temp Workspace

This folder contains only active unresolved or partial design decisions.

It can be deleted after all decisions are either:

1. moved into `docs/specs/` implementation contracts;
2. moved into ADRs;
3. marked intentionally out of scope.

## File

[`alignment-progress.md`](./alignment-progress.md) is the single active decision register. Completed decisions do not remain here; they move to canonical specs or ADRs and are recorded in `docs/agent/progress-log.md`.

## How to use

For each discussion round:

1. Pick the highest-risk unresolved area from `alignment-progress.md`.
2. Ask a focused batch of concrete questions with recommendations and tradeoffs.
3. Move accepted decisions into canonical specs or ADRs.
4. Remove the resolved entry from the active register.
5. Run deterministic and independent semantic gates.
6. Delete this folder when no active decisions remain.
