# Architecture Refinement Workflow

Use this when the user asks to improve code structure, package boundaries, or domain architecture.

## Steps

1. Read `docs/STRUCTURE.md`, `docs/specs/0001-repo-structure.md`, and task-relevant specs.
2. Identify the current boundary problem in concrete terms.
3. Check whether the issue affects product correctness, testability, security, or agent maintainability.
4. Propose the smallest structural change that improves the boundary.
5. Avoid abstractions that do not remove real complexity.
6. Add or update tests for the boundary.
7. Update specs only if the architecture contract changes.

Package and runtime boundaries are owned by `docs/specs/0001-repo-structure.md` and task-relevant canonical specs. Do not copy them into this workflow.
