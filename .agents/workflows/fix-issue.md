# Issue Fix Workflow

Use when the user asks to fix a filed issue. The issue text is a hypothesis, not a contract: verify it, fix the root cause completely, sweep sibling call sites, and escalate anything the issue got wrong or missed. A critical fixer never just implements the literal ask.

## Step 1 — Verify before you trust

1. Read the issue in full. List its **claims** (cited file:line, described defect, root cause) and its **fix plan**.
2. Verify each claim against the current working tree: read the cited files, reproduce the defect in code or in a test. If the app can run, reproduce visually.
3. Wrong claims happen. When the issue misdescribes a site (e.g. names a call site that is already compliant), do not "fix" it blindly: record the evidence and correct the issue in a comment and in the PR notes. Evidence over authority, always.
4. If the fix plan is unreasonable (wrong direction, over-scoped, contradicts a spec/ADR, or touches financial/security/irreversible data without the owner's say-so): do NOT implement it. State the objection with evidence in the issue thread and surface the blocker to the user (AGENTS.md: never implement through active blockers).

## Step 2 — Root cause, then complete fix

1. Find the root cause, not the symptom. A regression usually means a shared component lost a capability — fix the shared component, not one call site.
2. **Sibling sweep (举一反三)**: grep every consumer of the touched component/pattern (`grep -rn` the export, the pattern name, the class name). Every sibling with the same defect is fixed in the same PR, and the PR says so explicitly with the list.
3. Make the smallest correct change: no speculative abstraction, no refactor beyond the defect, follow the repo discipline gates (tailwind discipline, tokens, file-size ratchet, ui-discipline).
4. When a shared fix changes behavior for sites the issue never listed, list those sites in the PR as intentional coverage (with the issue link).

## Step 3 — Tests and gates

1. Add a regression test where the project has tests (markup/static assertions via `renderToStaticMarkup` for a11y phrasing, interaction tests for behavior). A fix without a regression test is a fix without a witness.
2. Run the issue's own verification steps plus the repo gate: `pnpm verify:fast` (or the task-appropriate subset, plus `cargo test`/`clippy` when Rust is touched).
3. For customer-facing UI, the final designer-level review stays with Kimi Code CLI (AGENTS.md item 9); note the handoff in the PR when the change is visual.

## Step 4 — Escalate what you find

During the fix, anything that is wrong but NOT part of this issue's root cause goes somewhere, never nowhere:

1. **Same defect class, more sites** → fix in this PR (Step 2).
2. **New, separable problem** (dead code, orphan props, spec drift, a second latent bug) → file a follow-up issue through the filing skill's dedup gate, or comment on the current issue when it is the same defect class. Link both directions (`Found while fixing #N` / `Follow-up: #M`) and mention the follow-up in the PR.
3. **The issue's own claims were wrong** → correct them in a comment on the issue, so the tracker's history tells the truth.

## Step 5 — Land it

1. Conventional branch name (`fix/…`, `chore/…`, `feat/…`), Conventional Commit subject, concise PR title/body per `.github/PULL_REQUEST_TEMPLATE.md`. No Jira linkage; no `gh-create-pr-from-branch`.
2. PR links the issue (`Fixes #N`); body lists divergences from the issue text with reasons.
3. Update `docs/agent/progress-log.md` with a dated entry (gates + scope).
4. When docs/`.agents`/`.codex`/`AGENTS.md` changed, run the independent semantic review gate (`.agents/docs-semantic-review.md`) — the reviewer must not be the patch author.
5. Close the issue after merge (or leave closing to the owner when they want to verify first).
