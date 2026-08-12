# Issue Filing Workflow

Use when the user asks to report a problem or when a review/fix session discovers one. Every issue is atomic, evidence-rich, dedup-checked, and never duplicates an open issue.

## Step 0 — Classify before searching

1. Read the finding and name its **component** (file/dir or feature area) and its **defect class** (a11y, layout/overflow, dead code, correctness, docs drift, motion, rhythm, copy, …).
2. Check `docs/specs/README.md` and status-bearing ADRs for the topic. A resolved or owned decision is not an issue: comment on the spec PR or fix the doc instead.
3. Check `docs/agent/progress-log.md` and `docs/agent/current-state.md` — if the topic is already tracked as a known deferral, comment on the tracking issue rather than filing a new one.

## Step 1 — Dedup gate (mandatory, never skipped)

1. List all open issues: `gh issue list --state open --limit 200 --json number,title,labels --jq '.[] | "\(.number)\t\(.title)\t\([.labels[].name]|join(","))"'`.
2. Search each of these keyword families against titles and bodies:
   - **Component vocabulary**: the component/area name and its synonyms (dialog, modal, spine, nav, SectionHeader, Sources, Review, Tasks, vault, website, parser, store, …).
   - **Defect-class vocabulary**: the symptom class (a11y, aria, checkbox, overflow, clipped, flash, dead code, orphan export, copy, docs, …).
3. Match verdicts:
   - **Same component + same defect class → duplicate.** Do NOT create an issue. Comment on the original with the new evidence (file:line, extra call sites, repro), so the original gains coverage instead of the tracker gaining noise.
   - **Same defect class + shared root cause across components → one issue** naming the root cause and listing every affected site (this is the 举一反三 direction: file the class, not the instance).
   - **Same component + different defect class, or independent root causes → separate issue.** Link related issues in each body.
   - **Original is closed and the defect re-appears, or the new finding is materially different (different root cause, different fix surface) → new issue**, and reference the closed one.
4. Record the search in the issue body: `**Dedup check**: searched open issues for <families>; no duplicate found` (or the comment target).

## Step 2 — Write the issue

Follow the class's canonical form — both templates are authoritative in
`.agents/workflows/issue-delivery.md` → Policy → Issue template and in the
GitHub forms:

- **Standard bugs, features, refactors, docs** → the six sections:
  Problem / Expected / Impact / Proposed scope / Verification / **Spec
  change** (`.github/ISSUE_TEMPLATE/bug_report.yml` for bugs,
  `feature_request.yml` for capability asks).
  - *Bug:* concrete repro steps, logs, app version; Spec change = the
    contract fix or `No spec change`.
  - *Feature:* the capability gap as Problem; **Spec change = the spec
    proposal** — the issue's first PR lands it together with the code.
- **Designer-level UI findings** → the `ui_review_finding.yml` form:
  Summary / Findings (numbered, each with `file:line` from the current
  working tree and the spec/standard reference, e.g. WCAG 2.5.8,
  `docs/specs/0008-design-system.md`) / Fix plan (minimal direction;
  where a shared-component fix exists, name the shared fix and list every
  call site that must move with it) / Verification (`pnpm verify:fast`,
  targeted tests, preview states, design-review handoff).
- Every standard issue ends with the `**Dedup check**: …` verdict from
  Step 1, so the tracker proves each issue earned its slot.

## Step 3 — Labels and hygiene

- Labels follow `issue-delivery.md` → Policy exactly: one primary
  category, one `priority:p0|p1|p2|p3` (impact rubric, never difficulty),
  one `scope:small|medium|large`; `ui-review` as the companion label for
  design findings. For UI findings the `ui_review_finding.yml` severity
  dropdown maps 1:1 onto the `priority:p*` label (P1 → `priority:p1`, …).
- Never put real financial data, account numbers, or secrets in an issue.
  Screenshots only of non-sensitive states.
- One issue = one root cause = one fix PR. If a finding bundles
  independent defects, split it before filing (or say explicitly that the
  bundled findings share one root cause).

## Step 4 — Close the loop

- If the finding came out of a fix/review session, reference the source PR in the issue body (`Found while fixing #N`) so reviewers can trace it.
- If you commented on an existing issue instead of filing, say so in the session summary — the tracker must not grow for the sake of growing.
