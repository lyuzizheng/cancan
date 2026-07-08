# Code Review Workflow

Use this when the user asks for review, risk analysis, or PR feedback.

## Stance

Lead with findings. Prioritize bugs, behavioral regressions, missing tests, security/privacy issues, and doc/spec divergence.

## Steps

1. Identify the changed files with `git diff --name-only` or the PR context.
2. Read relevant specs before judging behavior.
3. Check product invariants from `.agents/rules/product.md`.
4. Check data/security rules if touched.
5. Check tests and UI inspection evidence.
6. Return findings ordered by severity with file/line references.
7. If no findings, say so and name residual risk.

## Do Not

- Do not rewrite code during a review-only request.
- Do not flag style churn unless it creates risk.
- Do not suggest broad refactors unrelated to the change.
