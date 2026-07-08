# Development Cycle

Use this workflow for any non-trivial coding or documentation task.

## Loop

1. Run `.agents/scripts/agent-preflight.sh`.
2. Route the prompt with `.agents/ROUTER.md`.
3. Read the selected role and workflow.
4. Read the relevant canonical specs.
5. State assumptions and success criteria.
6. Make the smallest complete change.
7. Verify with the strongest available checks.
8. Update specs/current-state/progress when meaning changes.
9. Report what changed, what was verified, and what remains.

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
