---
name: cancan-testing-simulation
description: Design and run CanCan deterministic tests and simulated user/data flows. Use when the user asks for testing, QA, simulation, fixtures, flow validation, or regression coverage.
---

# CanCan Testing Simulation

## Flow

1. Pick a source-evidence scenario.
2. Define expected state transitions.
3. Use deterministic fixtures or mocked model outputs.
4. Reset the DB when data-layer code exists.
5. Compare source evidence, extracted records, review items, committed ledger, and Command Center summary.
6. Record gaps as tests or spec questions.
