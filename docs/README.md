# CanCan Finance Vault Docs

This folder is the product, architecture, and AI-agent operating manual for **CanCan**.

CanCan is not a traditional budgeting app. It is a local-first desktop finance vault that automatically collects financial evidence, parses it into canonical records, reconciles money movements across sources, and stores raw evidence plus derived records in an encrypted local vault.

## Current product definition

CanCan is a local-first financial evidence vault that turns fragmented financial sources into reconciled assets, transactions, positions, valuations, and money-flow links.

The center of the product is:

```text
Source -> Evidence -> Extract -> AI Normalize -> Validate -> Reconcile -> Review -> Ledger
```

Manual import exists as a test harness and fallback path. The product is successful only when the app can automate evidence collection, starting with Gmail statement discovery and download.

## Document map

| File | Purpose |
| --- | --- |
| [00-product-vision.md](./00-product-vision.md) | Product thesis, MVP success criteria, scope, non-goals |
| [01-system-architecture.md](./01-system-architecture.md) | Local-first Tauri/React/TypeScript/SQLite architecture |
| [02-domain-model.md](./02-domain-model.md) | Money sources, accounts, instruments, ledger events, match graph |
| [03-ai-parser-pipeline.md](./03-ai-parser-pipeline.md) | Native extraction + OCR + AI normalization pipeline |
| [04-reconciliation-engine.md](./04-reconciliation-engine.md) | Duplicate detection, transfers, card repayments, money-flow chains |
| [05-plugin-system.md](./05-plugin-system.md) | Gmail, manual import, watched folders, API/source plugins |
| [06-local-storage-security-backup.md](./06-local-storage-security-backup.md) | SQLite/SQLCipher, file vault, secrets, encrypted backup |
| [07-ui-information-architecture.md](./07-ui-information-architecture.md) | Command Center, Library, Review, Sources, Assets, Money Flow |
| [08-agent-and-llm-design.md](./08-agent-and-llm-design.md) | AI role, permissions, validation boundary, prompt/version logs |
| [09-technology-decisions.md](./09-technology-decisions.md) | Technology choices and tradeoffs |
| [10-roadmap.md](./10-roadmap.md) | Implementation phases after the docs are stable |
| [11-open-questions.md](./11-open-questions.md) | Resolved decisions and remaining questions |
| [agent/README.md](./agent/README.md) | How AI coding agents should use these docs |
| [agent/current-state.md](./agent/current-state.md) | Current product state, decisions, and next focus |
| [agent/progress-log.md](./agent/progress-log.md) | Persistent progress log for future iterations |
| [agent/iteration-protocol.md](./agent/iteration-protocol.md) | Required loop for docs/code consistency |
| [agent/consistency-checklist.md](./agent/consistency-checklist.md) | Checks before changing docs or code |
| [adr/0001-local-first-tauri-react-sqlite.md](./adr/0001-local-first-tauri-react-sqlite.md) | ADR: choose Tauri + React + SQLite |
| [adr/0002-agent-is-advisor-not-ledger-owner.md](./adr/0002-agent-is-advisor-not-ledger-owner.md) | ADR: AI proposes, deterministic engine/user commits |

## Non-negotiable principles

1. Every financial record must be traceable to source evidence.
2. AI may extract, normalize, propose, rank, and explain, but committed ledger state must pass deterministic validation.
3. The vault is local-first; CanCan does not require a hosted backend for MVP.
4. Gmail automation is in MVP; manual PDF import is a test path, not the success condition.
5. Docs, progress, and implementation must stay synchronized. See `docs/agent/` before coding.
