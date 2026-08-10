---
name: cancan-issue-triage
description: File, classify, and prioritize CanCan issues — write the five-section body plus the Spec change section, set one category, one priority, one scope, and decide split vs defer. Use when a problem or feature request surfaces and needs to become a tracked issue.
---

# CanCan Issue Triage

## Trigger

Apply this when work surfaces (bug report, feature request, failing test, review
finding). Features are issues too: a feature request is an `enhancement`-category
issue triaged exactly like a fix.

## Workflow

Run `.agents/workflows/issue-delivery.md` Stage 0. That workflow's **Policy**
section is the single canonical home for the body sections, category labels,
priority rubric, scope sizing, title convention, and split-vs-defer rule — do not
restate them here.

Before filing, run the mandatory **dedup gate** owned by the `cancan-issue-filing`
skill (`.agents/workflows/file-issues.md`): a near-duplicate becomes a comment on
the original, never a new issue. The spec is never written before its issue; a
feature issue's first PR lands the spec change and the implementation together.

## Stop And Ask

Ask before deciding money correctness, ledger semantics, irreversible
migrations, AI authority, secret handling, provider support, visual identity, or
backup compatibility — and before marking anything `priority:p0`.
