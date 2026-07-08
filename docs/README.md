# CanCan Docs

This folder is the product, architecture, and AI-agent operating manual for **CanCan**.

CanCan is a local-first desktop finance vault that collects financial evidence, parses it into canonical records, reconciles money movements across sources, and stores evidence plus records in an encrypted local vault.

## Canonical documentation model

CanCan docs are intentionally split into a small number of layers.

```text
docs/
  README.md                  entry point
  STRUCTURE.md               documentation rules and source-of-truth model
  specs/                     canonical product + technical specs
  adr/                       accepted architecture decision records
  agent/                     AI coding agent operating memory and workflow
  alignment-temp/            temporary grill/alignment workspace, deleted when done
```

## Source of truth

`docs/specs/` is the canonical implementation source of truth.

Older numbered product docs were removed to avoid duplicate and conflicting guidance. If an agent needs product/architecture detail, it should read the relevant spec rather than looking for `00-product-vision.md` style files.

Use this conflict order:

```text
1. User's latest explicit instruction in the active conversation
2. docs/agent/current-state.md
3. docs/specs/*.md
4. docs/adr/*.md
5. docs/alignment-temp/* only for active unresolved alignment
6. code behavior only when docs are silent
```

## Current product definition

```text
Gmail-first local finance evidence automation
+ AI-assisted parsing and normalization
+ deterministic validation
+ review-first reconciliation
+ local encrypted ledger and asset view
+ future AI assistant backed by narrow backend APIs/skills
```

Manual import exists as a test harness and fallback path. The product is successful only when the app can automate evidence collection, starting with Gmail statement discovery and download.

## Canonical specs

See [`specs/README.md`](./specs/README.md).

## AI agent entry point

Before implementation, an AI coding agent must read:

```text
1. docs/README.md
2. docs/STRUCTURE.md
3. docs/agent/current-state.md
4. docs/agent/reading-order.md
5. docs/agent/iteration-protocol.md
6. docs/agent/consistency-checklist.md
7. .agents/README.md
8. .agents/ROUTER.md
9. task-relevant docs/specs/*.md
10. task-relevant docs/adr/*.md
```

## Non-negotiable principles

1. Every financial record must be traceable to source evidence.
2. AI may extract, normalize, propose, rank, and explain, but committed ledger state must pass deterministic validation.
3. The vault is local-first; CanCan does not require a hosted backend for MVP.
4. Gmail automation is in MVP; manual PDF import is a test path, not the success condition.
5. MVP does not ask for base currency and does not default to one large Net Worth number.
6. Docs, progress, and implementation must stay synchronized through `docs/agent/`.
