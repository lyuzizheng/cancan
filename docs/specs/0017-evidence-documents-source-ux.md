# 0017. Evidence Documents and Source Detail UX Spec

## Goal

Define how CanCan exposes imported financial evidence in a personal finance app without becoming an enterprise reconciliation console.

Evidence should feel like part of each money source, not a separate corporate document-management system.

## Stable decisions

- Do not make Evidence Library a primary standalone sidebar section in MVP.
- Evidence lives under each Money Source detail view as a `Documents` or `Evidence` subview.
- The UI should feel polished, personal, and concise, not like an Ant Design/Excel admin table.
- Document detail does not require an always-visible embedded PDF preview in MVP.
- Source files open only in CanCan's in-app viewer; normal viewing does not create a plaintext temporary file or hand the original to an OS viewer.
- `Save a copy` is the explicit way to export a plaintext copy to a user-chosen location.
- Default document detail shows metadata at the top and records below.
- Technical extraction artifacts are hidden from normal UI.
- Document status should stay simple; detailed state belongs to jobs, review items, and parse runs.
- Search starts with local metadata/record search; full-text search can use SQLite FTS when needed.

## IA placement

Use Source detail as the home for evidence.

Recommended Source detail structure:

```text
Source Detail: DBS
  Overview
  Activity
  Documents
  Accounts / Cards
  Settings
```

`Documents` can later be globally searchable, but it should not be a dominant top-level product module in MVP.

## Personal-app UX guardrail

CanCan is for an individual managing personal money sources. The UX should be refined, calm, and visually designed.

Avoid:

```text
enterprise reconciliation console feel
plain Ant Design admin table look
Excel-like dense grids as the default
status-chip overload
technical pipeline labels in normal UI
large standalone document management section
```

Prefer:

```text
source-centric navigation
beautiful compact document rows
clear statement period and source identity
small record summaries
human action labels
subtle status indicators
smooth source/detail transitions
```

## Documents subview

The default Documents subview should be a polished list, not a heavy grid.

Each document row/card can show:

```text
document title or generated label
provider/source
statement period
amount of extracted records
primary account/container
imported at
last processed at
attention indicator only when needed
quick action: open / review / retry
```

Use visual grouping by month or statement period where helpful.

## Simplified document state

Do not model a long list of document statuses in the primary UI.

Persist only what is needed for safety and filtering. Let detailed state come from related jobs, parse runs, and review items.

Recommended user-facing states:

```text
Ready
Needs attention
Processing
File deleted
Missing
```

How they are derived:

```text
Ready             document has usable metadata/records and no blocking issue
Needs attention   locked PDF, parse failed, mapping needed, or review action exists
Processing        active job exists for this document
File deleted      the current encrypted Vault file was intentionally deleted; metadata and relationships remain
Missing           storage expected a current file but could not find or verify it
```

The technical reason can be available in detail, but the main UI should not expose every pipeline state.

Examples of hidden technical reasons:

```text
password_required
parse_validation_failed
account_mapping_needed
reconcile_candidates_pending
```

## Document detail layout

MVP detail view:

```text
Top metadata panel
  source
  document type
  statement period
  imported at
  related account/container
  view/save-copy action when the current file exists
  file-deleted state when only the registry/tombstone remains
  attention action if needed

Records section
  extracted/normalized records
  snapshots/balances if present
  linked review items when present
  linked ledger facts when committed
```

Do not require a PDF preview panel in MVP.

## Source file access

Provide `View document` where the current encrypted Vault file exists.

Rules:

```text
decrypt only inside the trusted Rust/Tauri boundary after Vault unlock
render requested pages into memory for the in-app viewer
send rendered page pixels, not the complete original file bytes, to the web UI
do not create a plaintext temporary file for normal viewing
release plaintext/page buffers on viewer close and Vault lock as far as the platform permits
never upload the file to a server merely for preview
show File deleted or Missing distinctly when the current file is unavailable
```

`Save a copy` opens the OS save picker and writes a normal plaintext file only to the location the user selects. The confirmation states that the saved copy is outside CanCan's encrypted Vault and becomes the user's responsibility. Cancelling or failing the export must not leave a partial destination file.

## Technical artifacts

Normal users should not see extraction internals.

Hide by default:

```text
native text extraction output
OCR layout JSON
raw AI response
validation JSON
prompt/model hashes
parse run internals
```

If needed later, expose them through a developer/debug panel, not the primary UX.

## Actions

Document detail and row actions may include:

```text
View document
Save a copy
Unlock
Retry processing
Re-run parser
Re-run matching
Review records
Delete source file
```

Use short human labels. Avoid exposing pipeline names such as `source_document_ingest` in the product UI.

## Delete source file behavior

`Delete source file` deletes the current encrypted file stored in CanCan's Vault. It is not Archive, staged-record removal, ledger Undo, or `Save a copy`.

The destructive confirmation explains:

```text
the current Vault file will be deleted
the document registry entry, record history, audit trail, and ledger links remain
linked views will show Source file deleted
future backups will not include the deleted file
older immutable backups or copies previously saved outside CanCan may still contain it
```

After confirmation, CanCan appends the deletion decision/audit event, removes the current encrypted blob, and retains the `source_documents` row as a tombstone. The row keeps its exact hash, metadata, semantic grouping, parse and record relationships, and ledger navigation. There is no Archive action in MVP.

Uncommitted linked records become ineligible for automatic commit and remain visibly associated with the deleted source; the user may separately remove those staged records. Committed ledger events and legs remain immutable. Correcting them requires the explicit reversal/replacement flow.

An exact-hash re-import reuses the tombstone and restores its current encrypted file. A byte-different file with the same semantic statement identity remains separate evidence under that statement identity.

CanCan guarantees application-level removal of the current Vault file, not forensic erasure from SSD wear-leveling, filesystem snapshots, or old backup media. A storage failure reports `Missing` rather than claiming that the user deleted the file.

## Search

### MVP search

Start with local structured search over:

```text
source/provider
account/container
document type
statement period
imported date
filename/generated label
record description/merchant
amount/currency
attention state
```

This can be implemented with normal indexed SQLite queries first.

### Full-text search

For broader search, use SQLite FTS5 inside the encrypted local database.

Recommended approach:

```text
create an FTS virtual table for safe searchable text
index document labels, source names, extracted record descriptions, merchants, and optional extracted text snippets
store row references back to source_documents and external_records
rebuild FTS from canonical tables when needed
```

Do not index secrets, statement passwords, OAuth tokens, AI keys, or raw sensitive debug payloads.

Raw full-document text indexing should be optional or delayed until there is a clear UX need. For MVP, searchable metadata and normalized records are enough.

## Evidence to ledger relationship

Document detail should show simple relationship summaries:

```text
records created
review items needing attention
ledger facts created
linked money flows
```

Any committed ledger event must be able to navigate back to its source document. The source document should also show which user-facing facts it produced.

Keep this visually simple. Use expandable sections rather than a graph-heavy audit UI.

## Empty states

Documents subview empty states should be source-specific.

Examples:

```text
No DBS statements yet. Connect Gmail or import a statement.
No Wise exports yet. Import a CSV/PDF export to start.
This source has documents, but none match the current filter.
```

## Import completion summary

After a Gmail, manual, folder, or API import completes, show a concise summary with expandable filenames:

```text
Imported
Already in CanCan
Looks like an existing statement
Source file restored
Needs attention
```

Exact duplicates do not create duplicate records. Re-importing an exact file whose current Vault copy was deleted restores that source document. A semantically matching file with different bytes remains available as separate additional evidence for the same statement identity.

## Acceptance criteria

- Evidence is accessed from Source detail, not as a dominant standalone sidebar section.
- Document UI feels personal, polished, and concise.
- An always-visible embedded PDF preview is not required, but `View document` opens the available source only in CanCan's memory-backed viewer.
- Normal viewing creates no plaintext temporary file and does not hand the original to an OS viewer.
- `Save a copy` is an explicit warned plaintext export to a user-selected location.
- Metadata appears above records in document detail.
- Technical extraction artifacts are hidden from normal UI.
- User-facing document states stay simple.
- `Delete source file` removes the current encrypted Vault file while retaining a navigable source-document tombstone and every record/ledger/audit relationship.
- There is no document Archive action in MVP; staged-record removal and committed-event Undo remain separate append-only actions.
- MVP search works through indexed structured fields.
- Future full-text search uses local SQLite FTS5, not a remote search service.
- Import completion identifies which files were new, already present, restored from a tombstone, or probable prior statements.
