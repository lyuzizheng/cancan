# Code Review Workflow

Use this when the user asks for review, risk analysis, or PR feedback.

## Inputs and readiness

1. Identify the implementation slice ID and review base.
2. Generate `.agents/scripts/implementation-review-packet.sh <slice-id> <base>`.
3. Require the external handoff named by the packet: exact request, assumptions and scope, success criteria, exact commands and results, and UI evidence when relevant.
4. Start only after the diff is frozen and the tester has passed every applicable gate required by `.agents/workflows/development-cycle.md`.
5. Use the packet's shared specs, ADRs, current state, blockers, full cumulative diff, and tracked/untracked file inventory.

## Correctness and safety gate

Check behavior, regressions, financial and data correctness, security and privacy boundaries, failure paths, unresolved blockers, test quality, UI evidence when relevant, and docs/spec consistency. Confirm that tests exercise the active production path rather than an obsolete or parallel implementation.

## Critical cleanup gate

Run this as a separate, mandatory pass over the entire changed subsystem, not only the displayed diff:

- Trace every changed artifact to the exact request and success criteria.
- Reject overengineering: speculative abstractions, configuration, indirection, extensibility, or defensive branches without a current requirement or consumer.
- For replacements and refactors, use `rg` to search for superseded paths and diff-created orphans: old names, consumers, duplicate implementations, files, exports, imports, dependencies, scripts, configuration, tests, docs, and lockfile entries. Confirm the old path is removed when the change made it obsolete. Report unrelated pre-existing dead code only as residual risk; do not expand scope to remove it.
- Check package boundaries and public APIs for accidental exports, dependency leakage, cycles, or abstractions used only once without a concrete boundary.
- Flag magic logic: unexplained constants, thresholds, ordering, positional coupling, string conventions, financial signs, identity rules, time rules, or security decisions. Require a named rationale and boundary/failure tests that hit the active path; reject tests that only preserve dead branches or mirror implementation details.

Each cleanup finding must cite the exact artifact, explain why it is unnecessary or unsafe, and propose the smallest deletion or simplification. Do not request unrelated broad refactors or style churn.

## Verdict and re-review

Return findings ordered by severity with file/line references, then one verdict:

```text
verdict: pass | changes_requested | blocked
```

`pass` requires no actionable P0/P1/P2 correctness or cleanup findings. `blocked` is reserved for missing evidence or a decision the reviewer cannot make. If changes are requested, the implementer owns the fix; all applicable evidence must be regenerated, and the reviewer must re-review the entire cumulative diff rather than only the latest fix.

When `.agents/docs-semantic-review.md` is triggered, return its semantic verdict separately; neither verdict substitutes for the other. If both passes have no findings, say so and name residual risk.

## Do not

- Do not edit files or review a patch authored by this reviewer when independence is required.
- Do not hide cleanup findings in residual risk.
- Do not suggest broad refactors unrelated to the request.
