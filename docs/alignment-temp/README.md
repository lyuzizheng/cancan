# Alignment Temp Workspace

This folder is a temporary alignment workspace for product, design, architecture, implementation, and AI-coding-agent workflow decisions.

It can be deleted after all decisions are either:

1. moved into numbered product/architecture docs;
2. moved into `docs/specs/` implementation contracts;
3. moved into ADRs;
4. marked intentionally out of scope.

## Files

| File | Purpose |
| --- | --- |
| [lifecycle-breakdown.md](./lifecycle-breakdown.md) | Complete product lifecycle question map |
| [alignment-progress.md](./alignment-progress.md) | Status table for aligned / partial / unresolved decisions |
| [grill-backlog.md](./grill-backlog.md) | Prioritized questions to ask the user |

## How to use

For each discussion round:

1. Pick the highest-risk unresolved area from `alignment-progress.md`.
2. Ask 5-10 concrete questions from `grill-backlog.md`.
3. Record user decisions in `alignment-progress.md`.
4. Move stable decisions into permanent docs/specs.
5. Delete this folder when no longer needed.
