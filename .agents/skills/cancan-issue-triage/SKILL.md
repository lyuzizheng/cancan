---
name: cancan-issue-triage
description: File, classify, and prioritize CanCan issues — write the five-section body plus the Spec change section, set one category, one priority, one scope, and decide split vs defer. Use when a problem or feature request surfaces and needs to become a tracked issue.
---

# CanCan Issue Triage

## When work surfaces (bug report, feature request, failing test, review finding)

Features are issues too: a feature request is an `enhancement`-category issue
triaged exactly like a fix.

1. **Run the Dedup gate first** — the full procedure lives in
   `cancan-issue-filing` (`.agents/workflows/file-issues.md`); verdicts:
   - Search open issues with **component × defect-class** vocabulary
     families (the affected area and synonyms × the symptom class).
   - **Same component + same defect class → duplicate**: comment on the
     original with the new evidence; never create a second issue.
   - **Same defect class + shared root cause across components → one
     issue** naming the root cause and listing every affected site.
   - **Different defect class or independent root causes → new issue**,
     linked to the related issue in both bodies.
   - **Closed + re-appearing or materially different → new issue**
     referencing the closed one.
   - Record the verdict in the issue body: `**Dedup check**: …`.
2. **Write the body** with the six sections from `.agents/workflows/issue-delivery.md` (Policy → Issue template):
   Problem / Expected / Impact / Proposed scope / Verification / **Spec change**.
   - *Bug:* concrete repro steps, logs, app version; Spec change = the
     contract fix or `No spec change`.
   - *Feature:* the capability gap as Problem; **Spec change = the spec
     proposal** — the new spec's outline or exactly which existing spec
     sections change. The spec is never written before the issue; the
     issue's first PR lands it together with the implementation.
3. **Title**: `<type>(<area>): <short imperative summary>` — e.g.
   `feat(import): dedupe attachment observations`,
   `fix(desktop): stale review counts after commit`,
   `refactor(parser): extract source-kind detection`.
4. **Labels**: exactly one primary category (`bug` / `enhancement` /
   `refactor` / `performance` / `security` / `infrastructure` /
   `documentation` / `test`), one `priority:p0`–`p3` (impact rubric, never
   difficulty — full table in `.agents/workflows/issue-delivery.md` Policy),
   one `scope:small` / `scope:medium` / `scope:large`.
5. **Split vs defer**:
   - `scope:large` → map to a slice in `docs/agent/implementation-slices.md`
     (or add one) and split into sub-issues before anything starts.
   - Defer when a product decision or dependency is missing; comment the
     reason, never silently.

## Stop And Ask

Ask before deciding money correctness, ledger semantics, irreversible
migrations, AI authority, secret handling, provider support, visual identity,
or backup compatibility — and before marking anything `priority:p0`.
