# Code Review Workflow

Use this when the user asks for review, risk analysis, or PR feedback.

## Stance

Lead with findings. Prioritize bugs, behavioral regressions, missing tests, security/privacy issues, and doc/spec divergence.

## Steps

1. Identify the implementation slice ID and review base.
2. Generate `.agents/scripts/implementation-review-packet.sh <slice-id> <base>`.
3. Use that packet's shared specs, ADRs, current state, blockers, and diff before judging behavior.
4. Check financial correctness, data/security boundaries, and unresolved implementation blockers if touched.
5. Check the slice's test gates and UI inspection evidence.
6. Return findings ordered by severity with file/line references.
7. If no findings, say so and name residual risk.

## Do Not

- Do not rewrite code during a review-only request.
- Do not flag style churn unless it creates risk.
- Do not suggest broad refactors unrelated to the change.
