# Consistency Checklist

Use this before considering a docs or code change complete. This checklist verifies ownership and evidence; it does not restate product behavior.

## Scope and authority

- [ ] Is the task's intended behavior owned by an existing entry in `docs/specs/README.md`?
- [ ] If no spec owns it, was a focused spec added and indexed without duplicating another topic?
- [ ] Were ADR claims interpreted according to the ADR's explicit status?
- [ ] Does `current-state.md` describe phase/current reality rather than override intended behavior?
- [ ] Are unresolved decisions present only in `docs/alignment-temp/alignment-progress.md` and as blockers in affected specs?
- [ ] Did the change avoid silently answering product, financial, security, privacy, or irreversible data questions?

## Implementation evidence

- [ ] Does app work use one ID from `docs/agent/implementation-slices.md`, with status/dependencies/blockers allowing implementation?
- [ ] Did implementation, testing, and review use the same generated slice context?
- [ ] Was the smallest justified execution tier used, with one production-code writer and only the independent roles that materially added evidence?
- [ ] Did the reviewer run separate correctness/safety and critical-cleanup gates over the entire cumulative diff?
- [ ] Did cleanup check task traceability, unjustified complexity, superseded paths, diff-created orphans, package/public API boundaries, magic logic, and whether tests hit the active path?
- [ ] Does code behavior match the relevant canonical spec, or is the divergence explicitly recorded?
- [ ] Do tests prove the changed behavior and important failure paths?
- [ ] Do financial/data invariants trace to their canonical spec and source evidence?
- [ ] Do security, connector, AI, secret, and backup changes satisfy their task-relevant specs/ADRs?
- [ ] Were implementation blockers respected rather than coded from directional prose?

## Verification evidence

- [ ] Did iterative work use focused checks, and did the final stable app diff run its applicable final app gate once after required code review?
- [ ] Did data-layer work use a safe test-only reset path?
- [ ] Did parser/LLM work use deterministic fixtures or mocked outputs according to `0016-testing-fixtures-agent-gates.md`?
- [ ] Did UI work include visual inspection and relevant state/accessibility checks once UI exists?
- [ ] Did testing use the narrowest sufficient layer while covering the full boundary of user-visible flows?
- [ ] Did `.agents/scripts/agent-preflight.sh` pass?
- [ ] If the harness changed, did `.agents/scripts/harness-self-test.sh` pass?
- [ ] If project agent configuration changed, did `.agents/scripts/check-codex-agents.sh` pass?
- [ ] After a review finding, were affected focused checks rerun and the entire cumulative diff reviewed again without repeating unrelated full builds?

## Documentation projection

- [ ] Did `docs/agent/current-state.md` change only if phase, focus, or current implementation state changed?
- [ ] Did `docs/agent/progress-log.md` get a dated entry only if the change altered a product decision, implementation scope, or architecture assumption (otherwise the PR body is the record)?
- [ ] Were resolved alignment entries removed after moving decisions to their canonical home?
- [ ] When the change-scope trigger in `.agents/docs-semantic-review.md` matched, did deterministic preflight/harness evidence run before an independent semantic reviewer returned `pass`?
