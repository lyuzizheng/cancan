# Plugin Guidance

This folder is for agent-facing plugin and capability guidance.

It is not the CanCan runtime plugin system. Runtime connector behavior belongs in `docs/specs/0003-gmail-collector.md`, `docs/specs/0004-parser-contract.md`, and future focused connector specs.

## Current Capability Contract

See `capability-contract.md`.

## Adding Plugin Guidance

Add a file here only when an agent needs repeatable instructions for a tool or connector that cannot live in a canonical product spec.

Examples:

- browser/Playwright visual inspection procedure;
- local OAuth development helper constraints;
- fixture redaction helper instructions.
