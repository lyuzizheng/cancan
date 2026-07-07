# 09. Technology Decisions

## Recommended stack

```text
Tauri + React + TypeScript + SQLite/SQLCipher
```

## Why not Rails/Sure as the app base?

Sure is useful as a reference for personal finance concepts such as accounts, entries, transactions, transfers, imports, trades, and valuations.

But CanCan's target product is different:

```text
local-first desktop app
SQLite vault
iCloud/folder backup
no server
no Postgres
no Redis
no hosted web deployment
reconciliation-first model
Gmail/local evidence automation
```

Use Sure as product/domain inspiration, not as the direct codebase.

## Why Tauri over Electron?

Tauri advantages:

```text
lighter desktop shell
smaller attack surface
explicit permissions/capabilities
Rust privileged layer
SQLite/plugin ecosystem
future mobile possibility
```

Electron advantages:

```text
all TypeScript/Node
fast prototyping
large ecosystem
```

Decision:

```text
Use Tauri for MVP unless Rust/Tauri friction becomes too high.
Keep business logic in TypeScript to reduce Rust complexity.
```

## Why TypeScript core?

```text
shared types with React UI
strong enough for business logic
excellent AI coding support
can share packages with future React Native mobile app
can keep parsers and reconciliation rules readable
```

## Why SQLite + SQLCipher?

```text
local-first
portable
single-user desktop app
works well with snapshot backup
easy export/inspection
no server required
encrypted at rest for sensitive financial data
```

## Why not make Agent the main engine?

Financial ledger mutation must be deterministic and auditable.

LLM output is useful but probabilistic. Therefore:

```text
Agent proposes.
Schema validates.
Rules score.
Policy/review confirms.
Ledger commits.
```

## When to introduce Python?

Introduce Python only if needed for:

```text
advanced OCR
PDF table extraction
local ML models
batch experiments
```

Python should be a sidecar worker with a strict input/output contract, not the primary ledger engine.
