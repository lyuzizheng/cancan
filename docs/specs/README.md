# Implementation Specs

This folder translates product docs into implementation-grade instructions for AI coding agents.

Read order:

1. `0001-repo-structure.md`
2. `0002-database-schema.md`
3. `0003-gmail-collector.md`
4. `0004-parser-contract.md`
5. `0005-review-and-commit-policy.md`
6. `0006-command-center-ui.md`
7. `0007-first-run-onboarding.md`
8. `0008-design-system.md`
9. `0009-backup-restore-versioning.md`
10. `0010-agentic-development-workflow.md`

Rules:

- Specs are not brainstorming notes. They are build contracts.
- If implementation diverges from a spec, update the spec in the same change.
- Each spec should define scope, non-goals, data contracts, validation, tests, and acceptance criteria.
- AI coding agents should use these specs to plan autonomous implementation loops.
