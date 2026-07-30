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

Production Tauri commands execute these pure functions through a separate request mode in the bundled Tauri-controlled Node worker. The core mode imports `packages/core`, accepts only bounded typed domain inputs, performs no AI/model call, and has no filesystem, database, network, secret, or renderer capability. Rust validates the protocol result and owns every SQLCipher transaction and durable mutation. This preserves one TypeScript implementation of financial rules without granting the renderer or the AI normalizer ledger-write authority.

### packages/db

Owns:

```text
migrations/*.sql
SQLite connection abstraction
portable migration/query tests
query benchmarks
index review notes
seed/reset helpers for tests
```

No Prisma. No ORM by default. Prefer explicit SQL plus typed repository functions.

The production desktop runtime includes these canonical migrations from `packages/db`, but the
SQLCipher connection, transactions, and narrow typed repositories live behind the Tauri/Rust
privileged boundary. The renderer receives product commands and read models, never a generic SQL
or database handle. The normalization sidecar remains database-free.

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
single-pass structured normalization as the initial runtime
bounded document-agent runtime adapters only after evaluation evidence justifies one
prompt/version logging helpers
model configuration types
AI permission boundary helpers
```

The document agent is a small normalizer with fixed parser tools. It is not a coding agent and must not expose shell, generic filesystem, arbitrary network, database, secret, or ledger tools.

The initial AI runtime is single-pass structured normalization in a trusted Node worker/sidecar bundled and controlled by Tauri. The same binary has a protocol-separated deterministic core mode so production host commands can execute `packages/core` without duplicating financial rules in Rust. Core mode performs no model or provider call and receives no filesystem, database, network, or secret capability. The existing macOS `arm64` evidence route uses the pinned Node 24 single-executable format. Phase 1 must package and validate an architecture-matched sidecar on both macOS `arm64` and `x86_64`; production signing and notarization remain release gates. The product must not require users to install Node, Docker, a VM, QEMU, or a separate sandbox runtime.

The Tauri/Rust boundary continues to own user-selected file access, SQLCipher transactions, durable jobs, and OS-secret retrieval. The sidecar is process separation and packaging, not an assumed permission sandbox. ToolLoopAgent and Pi Agent Core remain unselected until evaluation fixtures show a material advantage over the same single-pass contract.

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

## Source file size and module boundaries

Source files must stay small enough for one reviewer or agent to hold in context. `pnpm check:file-size` enforces the limits and runs inside `pnpm verify:fast` and `pnpm verify`:

```text
production .ts/.tsx/.rs    800 lines
*.test.* / *.spec.*        1200 lines
Rust tests.rs modules      2500 lines
```

A file that outgrows its limit is split along the boundaries below (or a newly documented boundary in this spec) before the change merges; the limit is never raised for convenience.

The privileged desktop host keeps domain-directory modules instead of single-file crates:

```text
apps/desktop/src-tauri/src/runtime/
  mod.rs             shared types/constants, VaultRuntime state and core accessors, re-exports
  error.rs           RuntimeError/VaultCommandError codes and the shared blocking-task helper
  keyring.rs         Keychain-backed remembered-key, statement-password, and bookmark stores
  sidecar.rs         normalizer/review-core sidecar process control and file-write helpers
  vault_lifecycle.rs vault create/unlock/lock/recovery methods and their commands
  inbox.rs           local-inbox methods, job orchestration, and their commands
  documents.rs       document/statement-password methods and their commands
  review.rs          review-ledger methods, batch jobs, and their commands
  tests.rs           runtime test module
apps/desktop/src-tauri/src/database/
  mod.rs             record types, ManualImportStore and its impl, re-exports
  migrations.rs      encrypted open, key derivation, migration application
  rows.rs            row-to-view mapping and review-record open helpers
  validation.rs      decimal/date/identifier validation helpers
  imports.rs         import/deletion persistence and audit helpers
  tests.rs           database test module
```

Splitting the `ManualImportStore` impl into per-aggregate modules is a registered follow-up in `docs/alignment-temp/alignment-progress.md`. Files that predate the guardrail (including that store impl, `app.tsx`, and the single-file `vault`/`viewer`/`source_observations` modules) carry ratchet-only per-file exemptions recorded in the `EXEMPTIONS` map of `scripts/check-source-file-size.mjs`; an exemption ceiling may only shrink, never grow, and removing one requires the split described here.

The store mutex protects only bounded state/repository reads, job claims, and transactional writes. File I/O, PDF/image decode, OCR, sidecar/model execution, hashing, and other long-running extraction work happen outside it. A worker snapshots the required IDs/version while holding the lock, releases it for the long work, then reacquires it and validates the current version/idempotency key before applying the result. The per-aggregate impl split remains a readability follow-up; long-work lock ownership is already decided and does not wait for that refactor.

The renderer keeps `app.tsx` as orchestration state only; presentational views live in sibling modules (`sources-view.tsx`, `document-modals.tsx`, `vault-gate.tsx`, `notices.ts`, `overview.tsx`, `review.tsx`, `vault-spine.tsx`). Interaction tests split per flow with shared fixtures instead of one monolithic file.

The presentation-safe Rust command surface generates its TypeScript request/response types into committed files. The native CI gate builds the required sidecar, reruns the focused Rust comparison test, and fails when its generated string differs from the committed file; the standalone check builds the sidecar itself before running that test. Semantic renderer API wrappers remain handwritten, and the current per-view React state/session guards remain in place. Do not add React Query, another cache framework, or a renderer state replatform merely to generate command types.

## Implemented workspace and application gates

The production workspace currently contains `apps/desktop`, `apps/website`, `packages/core`, `packages/db`, `packages/connectors`, `packages/parsers`, and `packages/ui`. The synthetic core slice implements pure financial preparation rules, the canonical proposal/grounding boundary, and a slice-owned SQLite migration/repository used only by deterministic tests. `packages/connectors` contains the deterministic Gmail Desktop OAuth/PKCE request, root-loopback callback, token-response, and mailbox-profile contract plus one privileged-use orchestration boundary. That boundary injects loopback binding, external-browser opening, JSON HTTP execution, and authorized-mailbox persistence; closes the listener before token exchange; and returns only normalized mailbox identity after persistence. Its reusable JSON executor has an injected `fetch` implementation, but the package has no embedded/live credential, credential loading/storage, concrete listener, Keychain or SQLite access, Tauri/renderer wiring, rule sync, provider-support claim, or production consumer. The existing renderer and privileged-host boundaries remain unchanged. The website package implements the static public surface from `0018`: React is a build-time authoring layer whose views are prerendered per route into HTML/CSS-only deployment output with no runtime JavaScript. A deterministic dist check covers internal links, accessibility structure, required disclosures, and the no-script boundary; the site has no backend, analytics, or tracking.

Root commands are real package scripts:

```text
pnpm dev
pnpm typecheck
pnpm test:unit
pnpm test:synthetic-core
pnpm check:rust
pnpm test:rust
pnpm build:web
pnpm build:website
pnpm check:website
pnpm build:desktop
pnpm verify:fast
pnpm verify:native
pnpm verify
```

`pnpm verify` remains the local application gate. Pull requests and `main` run the typecheck, production unit tests, the desktop web build, and the static-website build plus its content gate through `pnpm verify:fast` on Linux. `pnpm check:website` builds the site and then checks the emitted `dist/` for working internal links, required disclosures, and accessibility structure. A separate macOS workflow runs `pnpm verify:native` only for native desktop, sidecar/parser, migration, dependency, and pinned-toolchain inputs; it skips draft pull requests and can be invoked manually. The native gate uses frozen pnpm and Cargo lockfiles, restores only Cargo registry/git/target cache data, builds the sidecar once, and retains privileged Rust security/data-integrity tests, clippy, and the Tauri desktop build without charging renderer-only changes for a macOS runner. Standalone Rust and desktop-build commands remain self-contained. Production packages remain forbidden from importing the disposable spike.

## Acceptance criteria

- App can be built from root with one documented command.
- Tests can reset local database deterministically.
- `packages/core` tests run without Tauri, SQLite, or network.
- SQL migrations are versioned and repeatable.
- Docs mention every package that exists.
