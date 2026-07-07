# 01. System Architecture

## Recommended stack

```text
App shell:        Tauri
UI:               React + TypeScript
Core engine:      TypeScript packages
Privileged layer: Rust/Tauri commands
Database:         SQLite, preferably SQLCipher
File vault:       Content-addressed encrypted local files
Secrets:          OS secret storage / Tauri Stronghold / platform keychain
Backup:           Encrypted snapshot bundle to iCloud Drive folder
LLM layer:         TypeScript-first provider adapters and parser tools
Optional worker:  Python sidecar only for OCR/table extraction if needed
```

## High-level architecture

```text
React UI
  |
  | typed commands only
  v
Local Application Services
  - SyncRunService
  - ImportService
  - ParserService
  - ReconciliationService
  - ReviewService
  - BackupService
  |
  v
Core Domain Engine, TypeScript
  - source models
  - parser orchestration
  - normalization
  - dedupe
  - matching
  - validation
  - money-flow graph
  |
  v
Privileged Runtime, Tauri/Rust
  - SQLite open/backup
  - filesystem access
  - encryption helpers
  - secret storage
  - native dialogs
  - optional sidecar execution
  |
  v
Local Vault
  - finance.sqlite
  - files/<sha256>
  - backups
```

## Boundary rule

React is the UI only. It must not directly access arbitrary filesystem paths, secrets, or raw SQL.

Allowed UI actions:

```text
getHomeSummary()
listSources()
runPluginScan(pluginId)
openDocument(documentId)
createImportPreview(sourceDocumentId)
confirmReviewItem(reviewItemId)
rejectReviewItem(reviewItemId)
createBackupSnapshot()
```

Disallowed UI actions:

```text
readSecret("OPENAI_API_KEY")
fs.readFile("/any/path")
db.execute("DELETE FROM transactions")
commitLedgerWithoutReview()
```

## Runtime model

CanCan is a local app, not a server.

It still has a backend-like local engine, but there is no external backend, no hosted API, and no remote database.

```text
React -> local command API -> local engine -> SQLite/FileVault/Secrets
```

## Job execution

Use a SQLite-backed job engine instead of Redis or remote queue systems.

Jobs should be idempotent and resumable.

```text
jobs
- id
- job_type
- plugin_id
- target_id
- status: queued | running | paused | failed | completed
- current_step
- state_json
- input_hash
- output_hash
- retry_count
- lease_until
- created_at
- updated_at
```

On app start:

```text
1. unlock vault
2. load settings and enabled plugins
3. find unfinished jobs
4. mark expired running jobs as queued
5. build a run plan
6. show Continue Parsing / Run Scan options
7. execute jobs step by step with checkpoints
```

## Package layout

```text
cancan/
  apps/
    desktop/
      src/                  # React UI
      src-tauri/            # Tauri/Rust privileged layer
  packages/
    core/                   # pure TypeScript domain engine
    connectors/             # plugin implementations and shared interfaces
    parsers/                # statement parsers and AI prompts
    ui/                     # shared UI components
  docs/
```

## Local-first assumptions

- The user's local encrypted vault is the source of truth.
- iCloud is used for encrypted backup snapshots first, not live sync.
- Multi-device sync should be deferred until the desktop vault and backup format are stable.
- A future mobile app can initially be a read-only viewer for the latest iCloud backup snapshot.
