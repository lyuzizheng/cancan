# Design Grill Workflow

Use this to continue product/design alignment.

## Inputs

- `docs/agent/current-state.md`
- `docs/alignment-temp/alignment-progress.md`
- `docs/alignment-temp/grill-backlog.md`
- Task-relevant specs

## Steps

1. Identify the highest-risk unresolved or partial area.
2. Check if existing specs already answer the question.
3. Prepare a focused batch of 5-10 questions for normal grill sessions.
4. Use one-question mode only for security, money correctness, irreversible data shape, or a single blocking ambiguity.
5. Provide a recommended answer with tradeoffs for each question.
6. Record accepted decisions in `alignment-progress.md`.
7. Move stable implementation guidance into the canonical spec.
8. Remove or rewrite stale temp notes.
9. Update `docs/agent/progress-log.md`.

## Question Quality Bar

Each grill question should:

```text
name the decision
explain why it matters
recommend a default answer
make tradeoffs explicit
avoid overcomplicating the product
```

## Current Priority Order

1. Job engine and error model.
2. Testing/fixtures and agent automation gates.
3. Evidence Library detail UX.
4. Exact design token values and Figma prototype decision.
5. Future optional estimated-total/network-valuation policy.
