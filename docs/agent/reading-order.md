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

## Conflict handling protocol

If a conflict is found:

```text
1. Do not implement from the conflicting docs.
2. Classify the conflict using `docs/STRUCTURE.md` and read the relevant canonical spec or ADR.
3. If the intended answer is clear, update the stale document in the same change.
4. If the answer needs user judgment, record the blocker in `docs/alignment-temp/alignment-progress.md` and ask the user.
5. Record the cleanup in docs/agent/progress-log.md.
6. Run deterministic checks and independent semantic review.
```

## Completion gate

A task is not done until:

```text
code behavior matches docs
relevant docs match code behavior
progress log is updated
no known conflicts remain in touched topics
agent-preflight passes
independent semantic review passes when docs or harness files changed
```
