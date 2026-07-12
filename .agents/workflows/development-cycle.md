# Development Cycle

Use this workflow for any non-trivial coding or documentation task.

## Loop

1. Run `.agents/scripts/agent-preflight.sh`.
2. Route the prompt with `.agents/ROUTER.md`.
3. Read the selected skill and workflow.
4. For app work, select a slice and generate `.agents/scripts/context-for-slice.sh <slice-id>`.
5. Obey the packet readiness: `STOP` blocks coding; `EVIDENCE ONLY` permits only its named disposable spike/test work; `READY` permits implementation.
6. State assumptions and success criteria.
7. Make the smallest complete change.
8. Verify with the strongest available checks.
9. Update specs/current-state/progress when meaning changes.
10. If docs or harness files changed, run the independent semantic gate in `.agents/docs-semantic-review.md`.
11. Report what changed, what was verified, and what remains.

## Ask Instead Of Guessing

Ask the user when a decision affects:

- money correctness;
- ledger semantics;
- irreversible migrations;
- AI authority;
- privacy or secret handling;
- provider support claims;
- visual identity direction;
- backup/restore compatibility.
