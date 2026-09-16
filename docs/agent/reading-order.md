# AI Agent Reading Order

This file defines the canonical reading path for future AI coding agents.

The goal is to reduce duplicate interpretation, prevent stale-doc conflicts, and let an agent enter implementation mode with the smallest reliable context window.

## Default reading order

Before implementation, read in this order:

```text
1. docs/README.md
2. docs/STRUCTURE.md
3. docs/agent/current-state.md
4. docs/agent/reading-order.md
5. docs/agent/iteration-protocol.md
6. docs/agent/consistency-checklist.md
7. .agents/README.md
8. .agents/ROUTER.md
9. docs/specs/* relevant to the task
10. docs/adr/*.md relevant to the task
11. docs/alignment-temp/* only when active alignment decisions are needed
```

## Source contract

Use the concern-based source contract in `docs/STRUCTURE.md`. Do not reproduce it in agent or harness files.

## Canonical specs by topic

The maintained spec map lives only in `docs/specs/README.md`. Read the relevant entries from that index.

## App implementation context

Do not read all canonical specs for one coding task.

```text
1. Select a slice from docs/agent/implementation-slices.md.
2. Run .agents/scripts/context-for-slice.sh <slice-id>.
3. Use the packet's exact specs, ADRs, and blockers.
4. Obey `STOP`, `EVIDENCE ONLY`, or `READY`; evidence-only work is disposable and limited to the named spike/test gates.
5. Give implementation, testing, and review the same slice ID.
```

The slice manifest is current implementation planning, not a second product-spec layer. Canonical behavior remains in the referenced specs and status-bearing ADRs.

## Conflict handling protocol

If a conflict is found:

```text
1. Do not implement from the conflicting docs.
2. Classify the conflict using `docs/STRUCTURE.md` and read the relevant canonical spec or ADR.
3. If the intended answer is clear, update the stale document in the same change.
4. If the answer needs user judgment, record the blocker in `docs/alignment-temp/alignment-progress.md` and ask the user.
5. Record the cleanup in docs/agent/progress-log.md if it changes a product decision, implementation scope, or architecture assumption; otherwise capture it in the PR body.
6. Run deterministic checks and independent semantic review.
```

## Completion gate

A task is not done until:

```text
code behavior matches docs
relevant docs match code behavior
progress log gets a dated entry only when the change alters a product decision, implementation scope, or architecture assumption; otherwise the PR body is the delivery record
no known conflicts remain in touched topics
agent-preflight passes
independent semantic review passes whenever the change-scope trigger in `.agents/docs-semantic-review.md` matches
```
