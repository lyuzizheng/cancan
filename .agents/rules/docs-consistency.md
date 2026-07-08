# Docs Consistency Rules

- `docs/specs/` is the canonical implementation source of truth.
- `docs/agent/current-state.md` states current focus and decisions.
- `docs/alignment-temp/` is temporary unresolved alignment, not canonical truth.
- ADRs capture hard-to-reverse architecture decisions with tradeoffs.
- `.agents/` describes how agents work; it must not become a duplicate product spec layer.
- When behavior changes, update the relevant spec in the same change.
- When current focus or decision state changes, update `docs/agent/current-state.md`.
- For meaningful progress, update `docs/agent/progress-log.md`.
- Before finishing, run `.agents/scripts/check-docs-consistency.sh`.
