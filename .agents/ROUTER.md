# Agent Router

Use this file to choose a role and workflow from the user's prompt.

## Prompt Routing

| User asks for | Role | Workflow |
| --- | --- | --- |
| "grill", "stress-test", "align design", "questions" | `roles/design-griller.md` | `workflows/design-grill.md` |
| "implement", "build", "add", "fix" | `roles/implementer.md` | `workflows/implement-feature.md` |
| "review", "audit PR", "find risks" | `roles/code-reviewer.md` | `workflows/review-code.md` |
| "test", "simulate", "QA", "try flows" | `roles/qa-simulator.md` | `workflows/simulated-testing.md` |
| "refactor structure", "architecture", "boundaries" | `roles/architect.md` | `workflows/refine-architecture.md` |
| "polish UI", "improve design", "visual check" | `roles/ui-polisher.md` | `workflows/refine-ui.md` |
| "connector", "plugin", "Gmail", "source integration" | `roles/implementer.md` | `workflows/plugin-work.md` |

If a prompt contains several intents, pick the earliest blocker. For example, if the user asks to "grill the design then implement", run `design-grill` first.

## Role Rules

- Use one primary role per task.
- Mention assumptions before changing files.
- Ask only when the canonical docs do not answer a product, security, money-correctness, or irreversible data-shape question.
- Keep implementation slices small enough to verify independently.

## Quick CLI Helper

```bash
.agents/scripts/role-for-prompt.sh "review the parser and add tests"
```
