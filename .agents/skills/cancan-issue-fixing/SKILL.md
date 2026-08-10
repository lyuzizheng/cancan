---
name: cancan-issue-fixing
description: Fix GitHub issues for CanCan with a critical, evidence-first mindset. Use when the user asks to fix an issue, implement a fix, close an issue, 修issue, or when a filed issue needs resolution.
---

# CanCan Issue Fixing

## Workflow

Run `.agents/workflows/fix-issue.md` for the full loop. This skill exists so fixes verify the issue's claims instead of trusting them, cover every sibling call site, and escalate what they discover.

## Non-negotiables

1. **Verify before you trust.** The issue text is a hypothesis, not a contract. Reproduce the defect, confirm the cited file:line, and correct the issue publicly when its claims are wrong.
2. **Root cause + complete fix.** Fix the shared root cause, then sweep every consumer of the touched component for the same defect in the same PR.
3. **Push back with evidence.** Never silently implement an unreasonable fix plan. Deviations and objections go into the PR and the issue thread with evidence.
4. **Escalate what you find.** A fix that reveals a new, separable problem files a follow-up issue through the filing skill (dedup first) or comments on the current issue. A critical fixer leaves the codebase better documented, never just patched.
