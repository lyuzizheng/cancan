# 01. System Architecture

## Recommended stack

```text
App shell:        Tauri
UI:               React + TypeScript
Core engine:      TypeScript packages
Privileged layer: Rust/Tauri commands
Database:         SQLite, preferably SQLCipher from v1
SQL layer:        Hand-written SQL migrations + typed repositories, no Prisma
File vault:       Content-addressed encrypted local files
Secrets:          OS secret storage / Tauri Stronghold / platform keychain
Backup:           Encrypted snapshot bundle to iCloud Drive or user-selected folder
LLM layer:         TypeScript-first provider adapters, Vercel AI SDK allowed
Optional worker:  Python sidecar only for OCR/table extraction if needed
```

## High-level architecture

```text
React UI
  |
  | typed commands only
  v
Local Application Services / Assistant Skills
  - SourceSetupService
  - GmailCollectorService
  - ImportService
  - ExtractionService
  - ParserService
  - ReconciliationService
  - ReviewService
  - LedgerCommitService
  - AssetSummaryService
  - AssistantSkillService
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
askAssistant(question)
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

## Assistant APIs / skills

The future AI Assistant should access app data through narrow backend APIs/skills, not raw DB/files/secrets.

Examples:

```text
get_asset_summary()
get_monthly_summary(month)
list_review_items(status)
explain_money_flow(chain_id)
search_transactions(query)
get_source_freshness()
list_missing_statements()
```

Assistant APIs must enforce the same permission model as UI commands.

## Local-first and Gmail

Gmail collection does not make CanCan a hosted service. The app talks directly from the local desktop app to Google's read-only Gmail API after user consent. CanCan does not run a server, proxy, or remote database for MVP.

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

## Package layout

See `docs/specs/0001-repo-structure.md`.

## Documentation consistency rule

Any code change that changes product behavior, data model, source plugin behavior, parser output, AI authority, review policy, or roadmap state must update the relevant docs in the same PR/commit. The AI coding agent protocol in `docs/agent/` is part of the source of truth.
