# Testing Rules

Use the strongest verification available for the slice.

## Fixture Privacy

- Real statement samples belong in ignored `fixtures-private/`.
- Do not commit raw real bank/card/broker/insurance statements.
- Redacted fixtures require explicit manual redaction review before commit.
- Synthetic fixtures are preferred for deterministic CI.
- Do not paste fixture contents, account numbers, or statement screenshots into logs or reports.

## Before App Code Exists

- Run `.agents/scripts/agent-preflight.sh`.
- Run `git diff --check`.
- For docs-only changes, verify links/paths by search.

## Once App Code Exists

- Unit tests cover pure domain logic.
- Integration tests start from a reset test database for data flows.
- Test DB reset must only target test paths such as `.cancan-test/` or a temp directory.
- LLM-dependent tests use deterministic mocked model outputs or stored parse fixtures.
- UI changes require visual inspection with the best available browser/Chrome/Playwright/computer-use workflow.
- Hot SQL paths require deliberate indexes and lightweight benchmark coverage.

## Done Means

```text
tests pass
DB reset works if data layer changed
migrations apply if schema changed
parser fixtures pass if parsing changed
mocked AI outputs are deterministic if AI path changed
build succeeds if app code changed
UI was visually inspected if UI changed
docs/progress are updated
```
