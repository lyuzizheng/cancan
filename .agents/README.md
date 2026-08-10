# CanCan Agent Harness

This folder contains procedure only. Intended product and implementation behavior belongs in `docs/specs/`; architecture decisions belong in status-bearing ADRs.

## Start

1. Run `.agents/scripts/agent-preflight.sh`.
2. Follow `docs/agent/reading-order.md`.
3. Select the task-relevant skill and workflow from `.agents/ROUTER.md`.
4. For app work, select a slice from `docs/agent/implementation-slices.md`.
5. Generate the shared minimal context with `.agents/scripts/context-for-slice.sh <slice-id>`.
6. Obey the packet readiness: `STOP` blocks coding; `EVIDENCE ONLY` permits only its named disposable spike/test work; `READY` permits implementation.

## Shape

```text
.agents/
  README.md
  ROUTER.md
  docs-semantic-review.md
  workflows/   detailed task loops
  skills/      trigger-oriented entry points
  scripts/     deterministic gates, slice context, and review packets
```

Narrative role files, repeated product rules, static priority lists, generic templates, and placeholder plugin guidance are intentionally excluded. They created additional sources of truth without adding executable guarantees.

Project-scoped executable agent bindings live separately in `.codex/agents/`. Those small TOML files pin role instructions, model, reasoning effort, and only intentional subagent permission defaults. Implementer has no repo-local sandbox default; explorer/reviewer default to read-only and tester defaults to workspace-write. A parent turn's live permission selection is reapplied to every child and can supersede those defaults, so no-edit explorer/reviewer behavior is also a workflow contract rather than a hard isolation claim. Main-agent permissions stay user/session-owned rather than being copied into the repo. The workflow contract remains here and product behavior remains in `docs/specs/`.

## Two-layer gate

Every meaningful change uses both layers:

```text
deterministic gate
  -> paths, spec index, duplicate numbers, skill metadata, shell syntax,
     sensitive fixture tracking, stale harness references, whitespace

semantic gate
  -> an independent agent reviews changed docs and harness files for contradiction,
     hidden product decisions, stale status, unsafe implementation guidance,
     and duplicate ownership
```

Run the deterministic gate with `.agents/scripts/agent-preflight.sh`. Test the gate itself with `.agents/scripts/harness-self-test.sh`.

For docs, harness, or project agent configuration changes, prepare the semantic review with `.agents/scripts/docs-review-packet.sh HEAD` and follow `.agents/docs-semantic-review.md`. A semantic reviewer may identify a decision that needs user input, but must not decide it.

For app code, tests, or implementation review:

```text
.agents/scripts/context-for-slice.sh <slice-id>
.agents/scripts/implementation-review-packet.sh <slice-id> [base]
```

Every role used for a task shares the same generated slice readiness and canonical source index. Compact transport is not compact evidence: before implementation, testing, or review, each role opens the full contents of every indexed source from the exact working tree and head, then records that head and source list in its handoff. The index names exact files and headings without copying or summarizing their content; it never permits a role to skip canonical evidence. The implementation review packet likewise inventories the stable diff and evidence while the reviewer reads the complete cumulative diff from the shared repository. The slice checker validates dependencies, spec/ADR paths, active blockers, test gates, and outcomes.

Use the risk-sized paths in `.agents/workflows/development-cycle.md`, which is the canonical home for the Fast/Standard/High-risk tiers and their verification order. Use the read-only explorer only when separate structural analysis helps, and never run multiple source-writing agents concurrently.

Read implementation state from `docs/agent/current-state.md`; do not copy it into the harness. App commands may be added only when the referenced scripts and paths are real.
