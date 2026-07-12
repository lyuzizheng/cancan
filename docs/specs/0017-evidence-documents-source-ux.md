# 0017. Evidence Documents and Source Detail UX Spec

## Goal

Define how CanCan exposes imported financial evidence in a personal finance app without becoming an enterprise reconciliation console.

Evidence should feel like part of each money source, not a separate corporate document-management system.

## Stable decisions

- Do not make Evidence Library a primary standalone sidebar section in MVP.
- Evidence lives under each Money Source detail view as a `Documents` or `Evidence` subview.
- The UI should feel polished, personal, and concise, not like an Ant Design/Excel admin table.
- Document detail does not require embedded PDF preview in MVP.
- Users can open the original file when needed.
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
Removed
```

How they are derived:

```text
Ready             document has usable metadata/records and no blocking issue
Needs attention   locked PDF, parse failed, mapping needed, or review action exists
Processing        active job exists for this document
Removed           user removed the document from the active source view
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
  original file action
  attention action if needed

Records section
  extracted/normalized records
  snapshots/balances if present
  linked review items when present
  linked ledger facts when committed
```

Do not require a PDF preview panel in MVP.

## Original file access

Provide `Open original` where the local file exists.

Implementation blocker: the encrypted-vault-to-OS-viewer boundary is unresolved. Do not implement temporary plaintext extraction until the vault/file security design defines creation, permissions, cleanup, crash recovery, and audit behavior.

Rules:

```text
open local file with OS/default viewer
never upload it to a server for preview
show missing-file state if local file is unavailable
respect vault lock/encryption boundaries
```

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
Open original
Unlock
Retry processing
Re-run parser
Re-run matching
Review records
Remove
```

Use short human labels. Avoid exposing pipeline names such as `source_document_ingest` in the product UI.

## Remove behavior

The UI should present one simple action: `Remove`.

Implementation must still protect data integrity.

Committed ledger events and legs are immutable: `Remove` must not delete or rewrite them. Remaining Remove persistence and file semantics are unresolved, including proposal cleanup, source-document archive versus deletion, local file retention, evidence navigation, audit, and re-import. Do not implement those remaining behaviors until their canonical security/evidence owner defines them.

Do not expose multiple confusing actions like `remove from library`, `delete local file`, and `delete records` in the normal UI.

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
Already archived or removed
Needs attention
```

Exact duplicates do not create duplicate records. A semantically matching file with different bytes remains available as additional evidence for the same statement identity.

The summary reports an existing document's current archive/remove state; it does not define file deletion or retention semantics. Those remain in the Remove lifecycle blocker.

## Acceptance criteria

- Evidence is accessed from Source detail, not as a dominant standalone sidebar section.
- Document UI feels personal, polished, and concise.
- No embedded PDF preview is required for MVP.
- `Open original` exists when a local source file is available.
- Metadata appears above records in document detail.
- Technical extraction artifacts are hidden from normal UI.
- User-facing document states stay simple.
- `Remove` is one user-facing action while implementation preserves data integrity.
- Opening encrypted originals and the remaining Remove persistence/file behavior stay blocked until their canonical security and evidence semantics are accepted.
- MVP search works through indexed structured fields.
- Future full-text search uses local SQLite FTS5, not a remote search service.
- Import completion identifies which files were new, already present, probable prior statements, or already archived/removed.
