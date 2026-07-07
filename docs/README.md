# CanCan Finance Vault Docs

This folder is the project design library for **CanCan**, a local-first personal finance reconciliation vault.

CanCan is not a traditional budgeting app. It is a local desktop app that collects financial evidence from APIs, Gmail, PDFs, CSVs, screenshots, and manual uploads; parses those sources into canonical financial records; reconciles money movements across sources; and stores all raw evidence and derived records in an encrypted local vault.

## Document map

| File | Purpose |
| --- | --- |
| [00-product-vision.md](./00-product-vision.md) | Product definition, scope, non-goals, user loop |
| [01-system-architecture.md](./01-system-architecture.md) | Tauri + React + TypeScript + SQLite architecture |
| [02-domain-model.md](./02-domain-model.md) | Finance source, account, instrument, ledger event, match graph |
| [03-ai-parser-pipeline.md](./03-ai-parser-pipeline.md) | AI parser lifecycle from raw document to staged records |
| [04-reconciliation-engine.md](./04-reconciliation-engine.md) | Duplicate detection, transfers, top-ups, money-flow chains |
| [05-plugin-system.md](./05-plugin-system.md) | Source plugin system for API, Gmail, manual files, watched folders |
| [06-local-storage-security-backup.md](./06-local-storage-security-backup.md) | SQLite/SQLCipher, file vault, secrets, iCloud backup strategy |
| [07-ui-information-architecture.md](./07-ui-information-architecture.md) | UI pages, review inbox, assets view, source view, library |
| [08-agent-and-llm-design.md](./08-agent-and-llm-design.md) | How LLM/agent logic is used safely without owning the ledger |
| [09-technology-decisions.md](./09-technology-decisions.md) | Tech stack choices and trade-offs |
| [10-roadmap.md](./10-roadmap.md) | Phased implementation plan |
| [11-open-questions.md](./11-open-questions.md) | Product, architecture, and data questions to resolve |
| [adr/0001-local-first-tauri-react-sqlite.md](./adr/0001-local-first-tauri-react-sqlite.md) | ADR: choose Tauri + React + SQLite over Rails/Sure/SwiftUI |
| [adr/0002-agent-is-advisor-not-ledger-owner.md](./adr/0002-agent-is-advisor-not-ledger-owner.md) | ADR: LLM agents propose; deterministic engine/user commits |

## One-sentence product definition

CanCan is a local-first financial evidence vault that turns fragmented financial sources into reconciled assets, transactions, positions, and money-flow links.

## Core design principle

```
Source Document -> Parsed Record -> Canonical Ledger Event -> Match Graph -> Review -> Commit
```

Every record must be traceable back to source evidence. Every AI result must be validated before it changes the ledger.
