# UI Refinement Workflow

Use this when the user asks to improve visual quality, interaction quality, or UI design.

Frontend implementation is reserved to Kimi Code CLI (AGENTS.md item 9). Any other agent must stop here and direct the user to run this work through Kimi Code CLI.

## Steps

1. Read `docs/specs/0006-command-center-ui.md`.
2. Read `docs/specs/0008-design-system.md`.
3. Read `docs/specs/0011-visual-design-tokens.md`.
4. State the current UI goal and the user audience.
5. Make the smallest UI change that improves clarity or polish.
6. Verify responsive layout, text fit, empty/loading/error states, and reduced motion.
7. Inspect visually with the best available browser/Chrome/Playwright/computer-use tool once app code exists.
8. Update design specs only when the design contract changes.

Visual direction is owned by the referenced canonical specs. Do not copy it into this workflow.
