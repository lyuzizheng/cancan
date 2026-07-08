# Testing Rules

Use the strongest verification available for the slice.

## Before App Code Exists

- Run `.agents/scripts/check-docs-consistency.sh`.
- Run `git diff --check`.
- For docs-only changes, verify links/paths by search.

## Once App Code Exists

- Unit tests cover pure domain logic.
- Integration tests start from a reset test database for data flows.
- LLM-dependent tests use deterministic mocked model outputs or stored parse fixtures.
- UI changes require visual inspection with the best available browser/Chrome/Playwright/computer-use workflow.
- Hot SQL paths require deliberate indexes and lightweight benchmark coverage.

## Done Means

```text
tests pass
DB reset works if data layer changed
migrations apply if schema changed
build succeeds if app code changed
UI was visually inspected if UI changed
docs/progress are updated
```
