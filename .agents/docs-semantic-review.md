# Independent Docs Semantic Review

Use this gate after deterministic checks when a change touches anything under `docs/`, `.agents/`, or `.codex/`, root `AGENTS.md`, or the docs-harness CI workflow — **except** a change confined to append-only narrative memory (`docs/agent/progress-log.md` or `docs/agent/progress-log-archive.md`). Such a change needs only the author's self-check — the entry is dated, prepended, link-clean, and asserts no product claim that contradicts `docs/agent/current-state.md` or a spec — not an independent reviewer; the PR body carries the record. Any authority surface co-changed in the same patch re-arms the full gate.

The reviewer must not be the agent that authored the patch. If no independent reviewer is available, the semantic gate cannot return `pass`; report the gate as blocked.

## Inputs

1. The exact user request that authorized the patch.
2. The author's stated assumptions, scope boundary, and success criteria.
3. `docs/STRUCTURE.md` and `docs/agent/current-state.md`.
4. Changed files and diff from `.agents/scripts/docs-review-packet.sh <base>`.
5. Task-relevant canonical specs and ADRs.

The author must include inputs 1 and 2 in the reviewer handoff; the packet cannot infer authorization. If missing context prevents the reviewer from deciding whether the patch silently broadened scope, return `needs_design`.

## Judge checks

- One decision has one canonical owner.
- Summary, progress, alignment, and harness files do not override or duplicate product truth.
- New specs are indexed and old or placeholder specs are removed.
- Stable, partial, unresolved, and blocked language matches the actual decision state.
- The patch does not silently decide product, financial, security, privacy, or irreversible data questions.
- Implementation guidance is safe enough to execute, or an explicit implementation blocker is present.
- Current navigation, terminology, workflow, and priority claims do not conflict.
- Deleted files have no remaining actionable references.
- Each finding is scoped `introduced` (created or moved by this patch) or `pre-existing` (present on the base and outside the patch's changed lines). A `pre-existing` finding is a follow-up (a dedup-gated issue or comment), never a reason to widen this patch unless it shares the patch's root cause.

## Required output

```text
verdict: pass | fail | needs_design

findings:
- severity: P0 | P1 | P2
  scope: introduced | pre-existing
  evidence: file and line
  issue: concrete contradiction or risk
  action: mechanical fix or question for the user

residual_risk:
- anything deterministic checks cannot prove
```

Severity:

```text
P0  can corrupt authority, financial/data safety, or the harness gate itself
P1  can cause incorrect implementation, material drift, or a false pass
P2  clarity, maintainability, or non-blocking residual risk
```

Verdict precedence:

```text
fail          any mechanically actionable introduced P0/P1 remains
needs_design  a blocking introduced P0/P1 requires user judgment and no mechanical introduced P0/P1 remains
pass          only P2 findings, pre-existing findings, or no findings remain
```

Only an `introduced` P0/P1 can force `fail` or `needs_design`. A `pre-existing` finding never blocks this patch; record it as a follow-up and do not expand the patch to fix it unless it shares the patch's root cause.

The reviewer must not invent the answer to a `needs_design` finding.
