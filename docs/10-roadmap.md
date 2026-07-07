# 10. Roadmap

## Phase 0: Project skeleton

Goal: runnable local desktop app shell.

Tasks:

```text
Create Tauri + React + TypeScript project
Set up routing and layout
Set up local app data directory
Set up SQLite database and migrations
Set up secret storage abstraction
Create docs folder
Create initial test fixtures
```

Deliverable:

```text
App opens, creates/unlocks local vault, shows empty Home/Library/Settings pages.
```

## Phase 1: Vault and Library

Goal: store raw evidence locally.

Tasks:

```text
Create source_documents table
Create file vault storage by sha256
Manual PDF/CSV/image import
Document hash dedupe
Document list UI
Document detail UI
Basic metadata editor
```

Deliverable:

```text
User can drag files into Library and see stored evidence with hash and metadata.
```

## Phase 2: Parser foundation

Goal: parse text and structured records from imported documents.

Tasks:

```text
PDF text extraction
OCR fallback interface
parse_runs table
external_records table
structured JSON schema
parser validation pipeline
first generic CSV parser
first generic PDF text parser
```

Deliverable:

```text
User can parse a document into staged records without committing to ledger.
```

## Phase 3: First provider parser

Goal: one real provider works end-to-end.

Recommended first provider:

```text
One bank PDF/CSV statement or Wise export, depending available sample files.
```

Tasks:

```text
provider classifier
provider-specific parser
account mapping
balance/total validation
staged record preview
```

Deliverable:

```text
One real statement type becomes validated staged records.
```

## Phase 4: Ledger model

Goal: committed events and balances.

Tasks:

```text
finance_sources
accounts
instruments
ledger_events
ledger_legs
position/balance snapshots
commit staged records
ledger event list UI
asset summary UI
```

Deliverable:

```text
Parsed records can be committed and shown as assets/transactions.
```

## Phase 5: Reconciliation engine MVP

Goal: detect obvious duplicates and transfers.

Tasks:

```text
row hash dedupe
fingerprint dedupe
same-currency transfer matching
credit card payment matching
top-up matching
match_edges table
review_items table
Review Inbox UI
Confirm/Reject actions
```

Deliverable:

```text
User sees possible duplicates/transfers and can confirm or reject them.
```

## Phase 6: Money Flow UI

Goal: visualize linked chains.

Tasks:

```text
match graph query
chain builder
Money Flow page
source-to-source flow summary
unmatched endpoint view
```

Deliverable:

```text
User can inspect chains like Bank -> Wise -> Broker -> Trade.
```

## Phase 7: Gmail collector

Goal: ingest finance emails and attachments.

Tasks:

```text
Gmail OAuth read-only setup
email search rules
attachment download
raw email metadata storage
finance email filters
resume cursor/sync state
```

Deliverable:

```text
App can collect statement attachments from Gmail into Library.
```

## Phase 8: API connectors

Goal: read data from API-capable sources.

Priority:

```text
Wise
Moomoo
Bitget
```

Tasks:

```text
read-only token setup
sync state
overlap-window fetching
raw API JSON snapshots
API normalizers
connector health UI
```

Deliverable:

```text
App can import API records and reconcile them with documents/bank records.
```

## Phase 9: AI-assisted parsing

Goal: use LLMs for hard documents safely.

Tasks:

```text
AI provider settings
secret storage for provider keys
structured extraction prompts
schema validation
prompt/version logging
AI parse repair flow
AI match explanation
```

Deliverable:

```text
LLM can propose parsed records and explanations, but cannot commit ledger changes.
```

## Phase 10: Backup

Goal: encrypted local/iCloud backup.

Tasks:

```text
SQLite backup snapshot
file vault manifest
checksums
encrypted financevault bundle
iCloud folder selector/export
restore flow
backup history UI
```

Deliverable:

```text
User can create and restore encrypted vault backups.
```

## Later phases

```text
Mobile read-only viewer
review command sync from mobile
multi-device sync/event log
more bank parsers
insurance parser
advanced investment/crypto valuations
tax/capital gains reports
local model/Ollama support
parser marketplace/config sharing
```
