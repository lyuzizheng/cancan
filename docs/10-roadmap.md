# 10. Roadmap

## Roadmap principle

The roadmap should build toward the real product loop: automated evidence collection from Gmail, AI-assisted parsing/normalization, deterministic validation, reconciliation review, and committed ledger state.

Manual import can come first technically, but only as a test harness.

## Phase 0: Documentation and project skeleton

Goal: stable docs and runnable local shell.

Tasks:

```text
Finalize docs and agent iteration protocol
Create Tauri + React + TypeScript project
Set up routing and app layout
Set up local app data directory
Set up SQLite/SQLCipher database and migrations
Set up secret storage abstraction
Create initial test fixtures directory
```

Deliverable:

```text
App opens, creates/unlocks local vault, shows empty Command Center, Library, Sources, and Settings pages.
```

## Phase 1: Vault, sources, and manual import test harness

Goal: user can define Money Sources and store raw evidence locally.

Tasks:

```text
Create money_sources and accounts tables
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
User creates DBS/UOB/Wise sources and sub-accounts, imports a PDF/CSV, and sees stored evidence with hash and metadata.
```

## Phase 2: Gmail automation foundation

Goal: automatically collect statement evidence from Gmail.

Tasks:

```text
Gmail OAuth read-only setup
OS secret storage for Gmail refresh token
Gmail search rule UI
scan start date and incremental scan state
email/attachment metadata storage
attachment download into file vault
overlap-window scanning
scan status and error UI
```

Deliverable:

```text
User configures Gmail rules and CanCan downloads matching statement attachments into Library without modifying Gmail.
```

## Phase 3: Extraction and AI parser foundation

Goal: parse text/OCR into staged structured records.

Tasks:

```text
PDF native text extraction
OCR layer interface
extraction bundle storage
parse_runs table
external_records table
structured JSON schema
AI provider settings and provider adapter
prompt/version logging
parser validation pipeline
first generic CSV parser
first generic PDF/LLM parser
```

Deliverable:

```text
User parses a document into validated staged records without committing to ledger.
```

## Phase 4: First provider parsers

Goal: DBS/UOB/Wise samples work end-to-end.

Initial scope:

```text
DBS bank account statement
DBS credit card statement
UOB bank account statement
UOB credit card statement
Wise PDF/CSV/export
```

Tasks:

```text
provider classifiers
provider-specific prompt/templates
account mapping suggestions
statement period detection
balance/total validation
staged record preview
```

Deliverable:

```text
Real statements become validated staged records mapped to manually created sub-accounts.
```

## Phase 5: Ledger model and commit policy

Goal: committed events, legs, snapshots, and balances.

Tasks:

```text
instruments
ledger_events
ledger_legs
balance snapshots
valuation snapshots
commit staged standalone purchases by policy
manual review commit path
ledger event list UI
asset summary UI
```

Deliverable:

```text
Parsed records can become committed events and assets/transactions update correctly.
```

## Phase 6: Reconciliation engine MVP

Goal: detect obvious duplicates, transfers, repayments, and top-ups.

Tasks:

```text
row hash dedupe
fingerprint dedupe
same-currency transfer matching
credit card payment matching across banks
top-up matching
basic FX conversion matching
match_edges table
review_items table
Review Inbox UI
Confirm/Reject actions
```

Deliverable:

```text
User sees possible duplicates/transfers/repayments and can confirm or reject them.
```

## Phase 7: Command Center and Money Flow

Goal: make the product feel like a finance operations console.

Tasks:

```text
Command Center with source rail, review queue, evidence status, compact asset snapshot
match graph query
chain builder
Money Flow page
source-to-source flow summary
unmatched endpoint view
```

Deliverable:

```text
User can inspect chains like UOB -> DBS card repayment or DBS -> Wise -> Moomoo -> AAPL.
```

## Phase 8: Backup and restore

Goal: encrypted local/iCloud/folder backup.

Tasks:

```text
SQLite backup snapshot
file vault manifest
checksums
encrypted financevault bundle
iCloud/folder selector export
restore flow
backup history UI
```

Deliverable:

```text
User can create and restore encrypted vault backups.
```

## Later phases

```text
Wise API
Moomoo read-only connector
Bitget read-only connector
Manulife/insurance parser
advanced investment valuation
realized/unrealized gains
capital gains/tax reports
mobile read-only viewer
review command sync from mobile
multi-device sync/event log
local model/Ollama support
parser marketplace/config sharing
```
