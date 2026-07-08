# 0001. Repo Structure Spec

## Goal

Create a monorepo shape that lets AI coding agents implement, test, build, inspect, and iterate without mixing UI, domain logic, privileged desktop APIs, and parser logic.

## Target layout

```text
cancan/
  apps/
    desktop/
      src/                  # React app shell and pages
      src-tauri/            # Tauri/Rust privileged commands
      tests/                # app-level integration/e2e tests
  packages/
    core/                   # pure TypeScript domain engine
    db/                     # SQL migrations, query helpers, repositories
    connectors/             # Gmail/manual/API source plugins
    parsers/                # extraction bundles, parser contracts, prompts
    ai/                     # Vercel AI SDK adapters and structured extraction helpers
    ui/                     # reusable UI components and design tokens
    fixtures/               # redacted fixtures and expected outputs when approved
  docs/
    agent/
    specs/
```

## Package responsibilities

### packages/core

Pure TypeScript only. No filesystem, no Tauri, no secrets, no network, no direct LLM calls.

Owns:

```text
money source/account models
ledger event and ledger leg construction
validation rules
reconciliation scoring
money-flow graph building
base-currency calculation interfaces
```

### packages/db

Owns:

```text
migrations/*.sql
SQLite connection abstraction
repositories
query benchmarks
index review notes
seed/reset helpers for tests
```

No Prisma. No ORM by default. Prefer explicit SQL plus typed repository functions.

### packages/connectors

Owns source collection:

```text
manual import
Gmail read-only collector
watched folder import
future Wise/Moomoo/Bitget read-only APIs
```

### packages/parsers

Owns extraction and parse contracts:

```text
native PDF text extraction interface
OCR result interface
extraction bundle shape
provider parser interfaces
structured parser schemas
parser versioning
```

### packages/ai

Uses Vercel AI SDK where helpful.

Owns:

```text
provider routing
structured generation helpers
prompt/version logging helpers
model configuration types
AI permission boundary helpers
```

### packages/ui

Owns reusable product UI components, not page-specific business logic.

## Agent implementation rule

An AI coding agent should implement one vertical slice at a time and keep packages isolated. Example slice:

```text
migration -> repository -> core service -> UI page -> tests -> docs update
```

## Acceptance criteria

- App can be built from root with one documented command.
- Tests can reset local database deterministically.
- `packages/core` tests run without Tauri, SQLite, or network.
- SQL migrations are versioned and repeatable.
- Docs mention every package that exists.
