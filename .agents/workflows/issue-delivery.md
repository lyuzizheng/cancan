# Issue Delivery — the closed loop (issue → merged PR)

One issue, one loop, one PR. This workflow is the end-to-end executor for
**all** CanCan work — fixes, features, refactors, docs: every slice of work
enters as a GitHub issue and runs the same stages. A feature request is an
`enhancement`-category issue; its body proposes the spec change and its
**first PR lands the spec change and the implementation together**. The only
exception is the trivial no-behavior carve-out in `AGENTS.md` §10 (typo,
comment, formatting, dead link), which may ship without an issue.

`docs/specs/` owns intended product and implementation behavior; this
workflow owns the issue procedure only. Implementation mechanics stay in
`.agents/workflows/development-cycle.md` (risk tiers), `.agents/ROUTER.md`
(task routing), and `docs/agent/implementation-slices.md` (slice selection).

## Policy (canonical: this file)

### Categories (labels)

Every issue gets exactly **one** primary category label:

| Label | Use when |
| --- | --- |
| `bug` | Reproducible wrong behavior, crash, or data error |
| `enhancement` | New capability or visible enhancement (a feature) |
| `refactor` | Internal restructure with **no** behavior change |
| `performance` | Speed, latency, query, or resource use |
| `security` | Auth, data protection, secrets, privacy |
| `infrastructure` | CI, builds, releases, env/config, database operations |
| `documentation` | Docs, ADRs, spec work — no code behavior |
| `test` | Test coverage, test infrastructure, test-only fixes |

Companion labels are fine (`ui-review` findings, cross-cutting concerns), but
never two primary categories. **Features are issues too**: a feature request
is an `enhancement`-category issue running the exact same loop as a fix —
there is no "feature work without an issue".

### Priority (labels `priority:p0` … `priority:p3`)

Assign from the impact rubric — **not** from how easy the change looks:

| Label | Definition | CanCan examples |
| --- | --- | --- |
| `priority:p0` | Blocks a release, active data loss/corruption, or privacy/secrets exposure. Fix immediately, single-focus. | Vault unlock fails; ledger entries corrupted; evidence files written to the wrong source; secrets leaked in logs |
| `priority:p1` | A core flow is broken for real users; no acceptable workaround; or a regression on the happy path. | Import hangs; parser drops attachments; Review/commit flow fails; backup cannot restore |
| `priority:p2` | Degraded UX, correctness debt, or missing tests; normal schedule. | Wrong message on an edge case; slow query on a hot path |
| `priority:p3` | Polish, nice-to-have, backlog fodder. Never blocks anything. | Copy tweaks, optional refactors |

Deferral: an issue is deferred when it needs a product decision or a
dependency. Record the reason in a comment, never silently.

### Scope (`scope:small` / `scope:medium` / `scope:large`)

Estimate **before** starting, revise when the work lands:

| Label | Size | Guidance |
| --- | --- | --- |
| `scope:small` | ≤ half a day | One file or one concern; no plan needed |
| `scope:medium` | A few days | Touches multiple packages/apps or spec + code; brief plan in the issue |
| `scope:large` | Multi-session | Must be mapped to a slice in `docs/agent/implementation-slices.md` (or add one) and split into sub-issues before coding; never picked up whole |

### Issue template (five sections + Spec change)

Every standard bug/feature issue body uses exactly these sections (companion
forms such as `ui_review_finding.yml` may differ):

1. **Problem** — what is wrong or missing; concrete repro (steps, logs, version) for bugs; the capability gap for features.
2. **Expected** — the desired behavior, one sentence.
3. **Impact** — who is affected and how badly (feeds the priority rubric).
4. **Proposed scope** — files/packages/slices involved; what is explicitly out of scope.
5. **Verification** — how to prove the work (tests, acceptance criteria, gate names).
6. **Spec change** — how `docs/specs/` will change; **required whenever behavior changes**:
   - *Feature:* the spec proposal — the new spec's outline, or exactly which sections of an existing spec change. The spec is **not** written before the issue; the issue's first PR lands it together with the implementation.
   - *Bug:* the contract fix if the bug reveals a wrong behavior contract (which spec section), or `No spec change` when it does not.
   - *Refactor/docs/test:* `No spec change` unless behavior or authoritative docs move.

Title convention: `<type>(<area>): <short imperative summary>` — e.g.
`fix(desktop): stale review counts after commit`,
`feat(import): dedupe attachment observations`,
`refactor(parser): extract source-kind detection`. The `<type>` matches the
primary category (`fix`/`feat`/`refactor`/`perf`/`chore`/…).

### One issue → one PR

- Every PR references its issue: `Closes #<n>` in the description.
- A feature issue's **first PR lands the spec change and the implementation
  together** — the spec never changes in a separate PR before the code.
- PR review findings are fixed in the same branch. If the work grows beyond
  the issue's proposed scope, file a new issue instead of silently expanding.
- When the merged PR is the issue's implementing PR, the spec gained a
  `Tracked by: #<n>` section in that same PR (see below) — the issue closes
  and the link stays in the spec.

### Spec ↔ issue links (bidirectional, optional but recommended)

- **Issue → spec:** the issue's Spec change section points to the target
  spec/sections while the issue is open.
- **Spec → issue:** the merged spec carries a short optional section:

  ```text
  ## Tracked by

  - #<issue-number> (implementation tracked in GitHub; spec is the behavior authority)
  ```

  Add it only for the implementing issue of a feature; do not backfill old
  specs.

### Tracking rhythm

Every phase leaves a trace **on the issue** (a comment or the issue body
update): triaged (labels set), plan (spec-change sketch), implementation
start, review, merged. Anyone — human or agent — can reconstruct where a
unit of work stands from the issue timeline alone. `docs/agent/progress-log.md`
stays the local narrative log; the issue is the canonical status.

## The loop

### Stage 0 — Find / create the issue

**Artifact:** a triaged GitHub issue.

1. Run `.agents/scripts/agent-preflight.sh`.
2. Apply the `cancan-issue-triage` skill: search first; if none exists, write
   the six-section body (five sections + **Spec change**), set one category,
   one `priority:p0–p3`, one `scope:small–large`, title
   `<type>(<area>): <short imperative summary>`.
3. For a feature, the Spec change section is the **spec proposal**: new spec
   outline or the existing spec sections that will change. Do not write the
   spec now — the first PR lands it.
4. `scope:large` → map to a slice in `docs/agent/implementation-slices.md`
   (or add one) and split into sub-issues before coding.
5. Leave a triage comment recording category/priority/scope reasoning when it
   is not obvious.

### Stage 1 — Review the issue

**Artifact:** an issue whose scope and spec proposal are implementable.

- Re-read the issue against `docs/agent/current-state.md` and the affected
  spec sections; check the slice's packet readiness
  (`STOP` / `EVIDENCE ONLY` / `READY`).
- **Verify before you trust** (critical-fix doctrine, `.agents/workflows/fix-issue.md`):
  the issue text is a hypothesis — reproduce its claims against current
  code and correct the issue in a comment when a cited site is wrong or
  already compliant. Fix the shared root cause and sweep sibling call
  sites (举一反三) in the same PR; escalate new separable findings as
  follow-up issues (through `cancan-issue-filing`'s dedup gate) or
  comments, linked both ways.
- If the spec proposal or priority/scope is unclear, grill the reporter
  (`.agents/workflows/design-grill.md`) or comment on the issue — never infer
  unresolved product, financial, security, or irreversible data decisions.
- Comment the plan — this is the canonical implement checklist: files/packages
  touched, schema/migration impact, service/API impact, UI impact,
  test/fixture strategy, docs to update, the spec sections to change, and the
  execution tier from `development-cycle.md`.

### Stage 2 — Implement (test-first, spec + code together)

- For app work: select the slice, generate
  `.agents/scripts/context-for-slice.sh <slice-id>`, and open every listed
  source from the exact working tree and head.
- Write the spec change and the implementation in the **same working tree and
  the same PR**: update the canonical spec (`docs/specs/…`), then the code
  against it. For a new spec use `.agents/scripts/new-spec.sh`; keep the
  required spec format (Goal / Stable decisions / Data-API-UI behavior /
  Edge cases / Tests-acceptance criteria).
- Frontend behavior governed by the visual design specs is routed through
  Kimi Code CLI (AGENTS.md §9) — the issue tracks it, the executor is Kimi.
- Follow `development-cycle.md` verification order sized to the tier; run
  focused checks while editing, freeze the cumulative diff before required
  review, run the final app gate once after review passes.
- Update `docs/agent/current-state.md` and `docs/agent/progress-log.md` when
  meaning changes (development-cycle step 11).

### Stage 3 — Structural review

- Apply the change-scope trigger in `.agents/docs-semantic-review.md` with a
  reviewer that did not author the patch (`docs/`, `.agents/`, `AGENTS.md`
  changes); use `.agents/workflows/review-code.md` for code.
- Route findings back to the production-code writer; batch fixes into one
  writer pass. Do not push once per review comment.
- Comment the review outcome on the issue.

### Stage 4 — Pull request

- Open the PR with the template (`.github/PULL_REQUEST_TEMPLATE.md`):
  `Closes #<n>` in the Links section plus the spec section the PR changes.
  Keep the PR draft until the final CI evidence is green.
- Conventional title matching the issue: `feat(…)` / `fix(…)` / `refactor(…)` …
- The spec change is in this PR — never a separate spec-only PR before code.

### Stage 5 — Merge and close

- Before merge, make sure the implementing PR already added the optional
  `## Tracked by` section to the spec with the issue number when the
  feature's spec does not carry it yet (policy above) — it belongs in the
  implementing PR, never in a post-merge spec-only PR.
- Close the issue (`Closes #<n>` does it) and leave a final comment: merged
  PR, what shipped, what remains (follow-ups get their own issues). If the
  `Tracked by` section was missed, file a small follow-up documentation
  issue — never a spec-only PR without an issue.

Every stage leaves a trace **on the issue**: triaged → plan → implementation
start → review → merged (tracking rhythm is policy, above).
