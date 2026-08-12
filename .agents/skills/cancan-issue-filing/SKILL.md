---
name: cancan-issue-filing
description: File GitHub issues for CanCan with a mandatory dedup gate. Use when the user asks to report a bug, file an issue, create a finding, 提issue, or log a problem found during review or fixing.
---

# CanCan Issue Filing

## Workflow

Run `.agents/workflows/file-issues.md` for the exact filing procedure. This skill exists so every issue is atomic, evidence-rich, and never duplicates an open issue.

## Non-negotiables

1. **Dedup before create.** Every issue is preceded by an open-issue search over the repo's issue tracker. A near-duplicate means a comment on the original, never a new issue.
2. **Evidence over impression.** Every finding carries file:line references from the current working tree or a reproducible symptom. No speculation, no screenshots of secrets.
3. **One decision, one owner.** Check `docs/specs/` and status-bearing ADRs before filing; a spec/ADR that already owns the question is not a new issue — it is a comment or a doc fix.
