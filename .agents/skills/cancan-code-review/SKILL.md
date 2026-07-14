---
name: cancan-code-review
description: Review CanCan changes for correctness, safety, canonical-spec consistency, overengineering, refactor residue, package/API cleanliness, magic logic, and test validity. Use when the user asks for review, audit, PR feedback, risk analysis, or a cleanup pass after implementation.
---

# CanCan Code Review

## Workflow

Run `.agents/workflows/review-code.md` against the complete stable diff, task-relevant canonical specs, and status-bearing ADRs. Require passing test evidence before review and repeat the full review after fixes. Apply `.agents/docs-semantic-review.md` separately whenever its change-scope trigger matches.
