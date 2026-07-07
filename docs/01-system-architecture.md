# 01. System Architecture

## Recommended stack

```text
App shell:        Tauri
UI:               React + TypeScript
Core engine:      TypeScript packages
Privileged layer: Rust/Tauri commands
Database:         SQLite, preferably SQLCipher from v1
File vault:       Content-addressed encrypted local files
Secrets:          OS secret storage / Tauri Stronghold / platform keychain
Backup:           Encrypted snapshot bundle to iCloud Drive or user-selected folder
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
  - SourceSetupService
  - GmailCollectorService
  - ImportService
  - ExtractionService
  - ParserService
  - ReconciliationService
  - ReviewService
  - LedgerCommitService
  - BackupService
  |
  v
Core Domain Engine, TypeScript
  - source/account models
  - parser orchestration
  - AI normalization contracts
  - deterministic validation
  - dedupe and matching rules
  - ledger event/leg construction
  - money-flow graph
  |
  v
Privileged Runtime, Tauri/Rust
  - SQLite open/backup/encryption
  - controlled filesystem access
  - file vault read/write
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

React is the UI only. It must not directly access arbitrary filesystem paths, secrets, raw SQL, Gmail tokens, or ledger mutation primitives.

Allowed UI actions:

```text
getCommandCenter()
listSources()
createMoneySource()
createSubAccount()
configureGmailSearchRule()
runGmailScan(ruleId)
importDocumentManually(fileHandle)
openDocument(documentId)
createParsePreview(sourceDocumentId)
confirmReviewItem(reviewItemId)
rejectReviewItem(reviewItemId)
createBackupSnapshot()
```

Disallowed UI actions:

```text
readSecret("OPENAI_API_KEY")
readSecret("GMAIL_REFRESH_TOKEN")
fs.readFile("/any/path")
db.execute("DELETE FROM ledger_events")
commitLedgerWithoutValidation()
placeTrade()
makePayment()
```

## Local-first and Gmail

Gmail collection does not make CanCan a hosted service. The app talks directly from the local desktop app to Google's read-only Gmail API after user consent. CanCan does not run a server, proxy, or remote database for MVP.

Local-first means:

- parsed financial data lives in the encrypted local vault;
- attachments and extraction outputs are stored locally;
- Gmail tokens are stored in local OS secret storage;
- source documents are not sent to CanCan-owned servers;
- cloud AI usage is opt-in and controlled by user/provider settings.

Do not use computer-use/browser automation for Gmail in MVP unless the official API is blocked. Use read-only OAuth and explicit user-configured search rules.

## Job execution

Use a SQLite-backed job engine instead of Redis or remote queue systems. Jobs must be idempotent and resumable.

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
2. load settings, source definitions, and enabled plugins
3. find unfinished jobs
4. mark expired running jobs as queued
5. build run plan
6. show Continue / Run Gmail Scan / Review options
7. execute jobs with checkpoints
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
    connectors/             # Gmail/manual/API plugin implementations
    parsers/                # extraction, parser contracts, prompts
    ui/                     # shared UI components
  docs/
    agent/                  # AI coding agent memory and consistency protocol
```

## Documentation consistency rule

Any code change that changes product behavior, data model, source plugin behavior, parser output, AI authority, review policy, or roadmap state must update the relevant docs in the same PR/commit. The AI coding agent protocol in `docs/agent/` is part of the source of truth.
