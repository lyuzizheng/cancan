# Implement Feature Workflow

Implementing a slice is **Stage 2 of the issue loop**. The canonical steps and
the plan checklist live in `.agents/workflows/issue-delivery.md` (Stages 1–2),
and the verification order lives in `.agents/workflows/development-cycle.md`.
This file is the operational reminder; it does not restate the loop.

1. Ensure a tracked issue exists (or the trivial carve-out in `AGENTS.md` §10 applies).
2. Run `.agents/scripts/agent-preflight.sh`.
3. Select one slice ID from `docs/agent/implementation-slices.md` and generate
   `.agents/scripts/context-for-slice.sh <slice-id>`; open every listed source in
   full from the exact working tree and head, and record that head and source list.
4. Obey the packet readiness: `STOP` blocks coding; `EVIDENCE ONLY` permits only
   the named disposable spike/test work and no production code; `READY` permits
   implementation. If no slice owns the behavior, update the slice plan first.
5. Make the smallest complete change (spec + code together when behavior changes),
   own focused tests for the changed behavior and its failure paths, and verify
   sized to the tier in `development-cycle.md`.
6. For any required independent review, freeze the cumulative diff and generate
   `.agents/scripts/implementation-review-packet.sh <slice-id> <base>`.

One production-code writer; never run source-writing agents concurrently.
