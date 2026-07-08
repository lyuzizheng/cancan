# 09. Technology Decisions

## Recommended stack

```text
Tauri + React + TypeScript + SQLite/SQLCipher
```

## AI SDK decision

Vercel AI SDK may be used for provider routing and structured generation helpers.

The app should still wrap it in CanCan-owned adapters so core parser logic does not depend directly on provider-specific APIs.

## SQL decision

Use hand-written SQL migrations and typed repository/service functions.

Do not use Prisma.

Reasons:

```text
SQLite/SQLCipher control
predictable performance
easier index design
clear migrations for local vault upgrades
less ORM magic around financial data
```

SQL guidance:

```text
avoid huge clever SQL for business logic
prefer small indexed queries and TypeScript composition when clearer
benchmark hot paths
use JSON fields for provider-specific/evolving metadata
promote JSON fields to columns when they become frequent filters/sorts
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
