# 0001. Repo Structure Spec

## Goal

Create a monorepo shape that lets AI coding agents implement, test, build, inspect, and iterate without mixing UI, domain logic, privileged desktop APIs, and parser logic.

## Accepted architecture evidence

ADR 0001 is accepted. The [disposable feasibility spike](../../spikes/desktop-feasibility/EVIDENCE.md) proved a Tauri 2 desktop build, bundled SQLCipher with FTS5, and the Rust privileged boundary on macOS arm64. Exact vault cryptography and release targets remain owned by their focused specs and do not block creation of the package skeleton.

## Target layout

```text
cancan/
  AGENTS.md                 # tool-neutral coding-agent entry point
  .codex/                   # project-scoped executable agent and model bindings
  .node-version             # pinned Node.js LTS version
  rust-toolchain.toml       # pinned Rust plus clippy/rustfmt
  .agents/                  # skills, workflows, deterministic and semantic gates
  scripts/                  # one-command developer setup and its deterministic tests
  apps/
    desktop/
      src/                  # React app shell and pages
      src-tauri/            # Tauri/Rust privileged commands
      tests/                # app-level integration/e2e tests
    website/                # static landing, privacy, security, help, and download surface
  packages/
    core/                   # pure TypeScript domain engine
    db/                     # SQL migrations, query helpers, repositories
    connectors/             # Gmail/manual/API source plugins
    parsers/                # extraction bundles, parser contracts, prompts
    ai/                     # Vercel AI SDK adapters and structured extraction helpers
    ui/                     # reusable UI components and design tokens
    fixtures/               # redacted fixtures and expected outputs when approved
  spikes/                   # disposable architecture/security evidence; never a production dependency
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
source-backed valuation and compatible subtotal interfaces
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

Owns document-normalization contracts and product parser skills:

```text
native PDF text extraction interface
OCR result interface
extraction bundle shape
job-scoped document-agent tool interfaces
versioned provider/document parser skills
structured parser schemas
parser versioning
```

Product parser skills are runtime artifacts for supported documents. They must not load or depend on the repo-development skills under `.agents/skills/`.

### packages/ai

Uses Vercel AI SDK where helpful.

Owns:

```text
provider routing
single-pass structured normalization
bounded document-agent runtime adapters
ToolLoopAgent / Pi Agent Core comparison boundary until the runtime spike selects one
prompt/version logging helpers
model configuration types
AI permission boundary helpers
```

The document agent is a small normalizer with fixed parser tools. It is not a coding agent and must not expose shell, generic filesystem, arbitrary network, database, secret, or ledger tools.

Both current runtime candidates require Node 22 or newer. Production execution and packaging remain evidence-gated: the preferred candidate is a trusted Node worker/sidecar bundled and controlled by Tauri, while renderer execution may be accepted only if the spike proves compatibility without exposing OS secrets or broad Tauri capabilities. The product must not require users to install Node, Docker, a VM, QEMU, or a separate sandbox runtime.

The Tauri/Rust boundary continues to own user-selected file access and OS-secret retrieval. A sidecar, if selected, is process separation and packaging, not an assumed permission sandbox.

### packages/ui

Owns reusable product UI components, not page-specific business logic.

### apps/website

Owns the static Cloudflare Pages surface defined by `0018`. It may reuse tokens and present verified release metadata, but it must not become a hosted CanCan backend or own desktop product behavior.

## Agent implementation rule

An AI coding agent should implement one vertical slice at a time and keep packages isolated. Example slice:

```text
migration -> repository -> core service -> UI page -> tests -> docs update
```

Production packages must not import from `spikes/`. A spike may remain as reproducible evidence until equivalent production tests exist.

Developer setup is a root concern. `scripts/setup-dev.sh` installs only the pinned toolchain and invokes real package/harness commands; it must not duplicate product behavior or hide package-specific build logic.

## Implemented workspace and application gates

The production workspace currently contains `apps/desktop`, `packages/core`, `packages/db`, `packages/parsers`, and `packages/ui`. The synthetic core slice implements pure financial preparation rules, the canonical proposal/grounding boundary, and a slice-owned SQLite migration/repository used only by deterministic tests. The desktop shell imports the reusable UI package and still exposes no network, vault, secret, database, or Tauri command capability.

Root commands are real package scripts:

```text
pnpm dev
pnpm typecheck
pnpm test:unit
pnpm test:synthetic-core
pnpm check:rust
pnpm build:web
pnpm build:desktop
pnpm verify
```

`pnpm verify` is the local application gate. The macOS application workflow installs from the pinned toolchain files, uses frozen pnpm and Cargo lockfiles, runs repository preflight, and then runs that same gate. Production packages remain forbidden from importing the disposable spike.

## Acceptance criteria

- App can be built from root with one documented command.
- Tests can reset local database deterministically.
- `packages/core` tests run without Tauri, SQLite, or network.
- SQL migrations are versioned and repeatable.
- Docs mention every package that exists.
