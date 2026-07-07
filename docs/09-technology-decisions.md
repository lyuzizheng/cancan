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
iCloud backup
no server
no Postgres
no Redis
no hosted web deployment
reconciliation-first model
```

Rails/Sure is a web/server app shape. CanCan is a local encrypted vault.

Use Sure as product/domain inspiration, not as the direct codebase.

## Why not SwiftUI?

SwiftUI is strong for Apple-native apps, but the current developer skillset favors React, backend frameworks, and Flutter. The project also expects heavy coding-AI usage, where TypeScript/React tends to be easier to generate, review, and refactor for this team.

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

## Why SQLite?

```text
local-first
portable
single-user desktop app
works well with snapshot backup
easy export/inspection
no server required
```

## Why SQLCipher or equivalent encryption?

Financial records, statements, account balances, PDFs, and API snapshots are highly sensitive. The default should be encrypted local storage.

## Why not make Agent the main engine?

Financial ledger mutation must be deterministic and auditable.

LLM output can be useful but is probabilistic. Therefore:

```text
Agent proposes.
Schema validates.
Rules score.
Review confirms.
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

## Initial architecture decision records

See:

```text
adr/0001-local-first-tauri-react-sqlite.md
adr/0002-agent-is-advisor-not-ledger-owner.md
```
