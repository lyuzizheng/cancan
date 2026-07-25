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
- CanCan is inbox-first rather than Gmail-first: every acquisition channel converges on the same encrypted evidence, classification, parsing, and review path.
- Users may add evidence without choosing a source/account first; trusted classification and account resolution perform routing, with one compact question only when ambiguous.
- The initial automatic phone-to-Mac path is a user-authorized iCloud Drive `Cancan` root. CanCan does not scan all of iCloud Drive or Downloads, or move/delete source files.
- A future iOS Share Extension is a thin intake companion, not a mobile ledger or a prerequisite for MVP.

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

Global `Add` actions, drag/drop, and Open With may accept evidence from anywhere in the app. Evidence that is still unassigned appears as one `Needs attention` card in Command Center until classification or a compact source choice resolves it; this is not a second document library.

## Acquisition channels

All channels create the same `source_documents` evidence artifact and then run the same trusted classifier, account resolver, parser, and reconciliation policy.

MVP order:

```text
1. Add files, drag/drop, and Open With CanCan
2. optional user-authorized CanCan iCloud Drive root, using its `Inbox` child
3. optional user-authorized Gmail rules, including send-to-self attachments
4. AirDrop or any cloud-drive app, followed by Add/Open With or `Cancan/Inbox`
```

Do not build separate Dropbox, OneDrive, iCloud Drive, or AirDrop connectors. The initial automatic-folder contract below is only for the user-authorized iCloud Drive `Cancan` root; it is not a whole-iCloud scan or a provider-generic setup surface.

### User-authorized CanCan iCloud Drive root

Through the system folder picker, the user explicitly authorizes one iCloud Drive `Cancan` root. CanCan's only app-owned child names below that root are:

```text
Inbox
Backups
```

Existing directories with those names are reused; missing directories are created. If either name is occupied by a non-directory, setup fails with an actionable state. CanCan never overwrites, moves, or deletes user data to resolve that conflict.

The user puts supported statements only in `Inbox`. Future local automation observes only that child, never the root itself or `Backups`. `Backups` is reserved for CanCan backup outputs and is never an ingestion source. Creating `Backups` means only that the target is prepared; it does not enable, schedule, or prove a backup. Backup bundle, scheduling, and restore behavior remain owned by [0009](./0009-backup-restore-versioning.md) and the blocked `backup-release` slice.

The user may later change or disable the root. This contract does not invent migration or cleanup behavior, and CanCan never deletes old roots or their child directories.

The root and its children remain outside the encrypted Vault and follow iCloud Drive's privacy and security model. The readiness evidence selects a native preflight-before-Rust-open rule, but does not implement it in production. Production still needs its own folder-picker authorization/bookmark, native integration, and watcher UI.

The root gives a simple phone flow:

```text
bank app Share
-> Save to Files
-> Cancan/Inbox
-> Mac sync provider makes the file readable
-> CanCan captures it into the encrypted Vault
```

Rules:

```text
observe only the authorized root's Inbox child
scan on enable, app startup/unlock, manual refresh, and filesystem-change hints while the app runs
accept only supported regular PDF/CSV/image files that can be opened read-only
ignore directories, symlinks, hidden/temp/partial-suffix files, and unsupported types
observe the same file identity, size, and modification time across two scans separated by a fixed two-second settle interval
after reading/hash, re-stat the file; if identity, size, or modification time changed, discard the bytes and retry later
validate the supported container/header before Vault registration
defer cloud placeholders, provider/offline errors, changing files, and unreadable files; retry on a later scan
before Rust opens or reads an iCloud candidate, native preflight allows only `current`; `notDownloaded`, `downloaded` (possibly stale), unknown/nil, downloading, and provider errors defer
use SHA-256 import idempotency, so rescans and duplicate channels are safe
skip hashes whose Vault artifact was user-deleted; only an explicit Restore/Add confirmation may restore them
copy into the encrypted Vault before processing
never modify, move, rename, or delete the user's source file in MVP
never observe the root itself, Backups, or any other iCloud Drive location
show that the selected root remains outside the CanCan Vault and follows iCloud Drive's privacy/security
```

Filesystem notifications are a wake-up hint, not the source of truth; deterministic `Inbox` rescans provide recovery after sleep, app exit, or sync delay. Do not watch the whole Downloads folder by default.

The two-second settle interval is a capture-protocol constant, not a user setting. The completed [local-inbox readiness evidence](../../spikes/local-inbox-readiness/EVIDENCE.md) proves this protocol against an ordinary local folder and the supported `Cancan/Inbox` iCloud Drive path, including a continuously changing file that never reaches capture early, a native placeholder preflight before Rust access, and a fresh-process restart lifecycle. Production folder automation remains a later slice.

### Email send-to-self

A phone user may Share to Mail and send a PDF/CSV to their own authorized Gmail account. The generic Gmail Inbox rule captures the attachment and trusted classification routes it. CanCan does not operate an inbound email service in MVP.

### Future phone Share Extension

A real `Share to CanCan` action requires an iOS containing app because an App Store Share Extension ships inside an app target. The future product should be a thin native Swift intake companion plus Share Extension, not a second finance UI.

The extension contract is deliberately small:

```text
accept supported PDF/CSV/image items from the system Share sheet
validate type and size, compute SHA-256, and persist one immutable handoff manifest
return a truthful Saved / Needs app to finish status quickly
never parse, call AI, mutate the ledger, or receive the desktop Vault master key
```

The extension and containing app may use an App Group for short-lived local handoff. Cross-device transport, encryption before cloud transit, background completion, retry, deletion, and pairing/recovery must be proven in a disposable local Xcode feasibility slice before choosing between a user-visible iCloud folder and an encrypted paired handoff. Do not embed this work in the generated Tauri desktop project or promise background sync before that evidence exists. Production signing, TestFlight, and App Store release belong to a later separately authorized release slice.

Hosted upload email, local-network upload pages, bank-app automation, and provider-specific cloud-drive APIs remain deferred because they add custody, availability, or integration surface without improving the first simple path.

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

For canonical email-message evidence, show sender, subject, message date, provider, and the normalized record summary instead of inventing a filename or PDF preview. Display only the bounded body evidence retained under the user's transaction-notification consent.

## Source file access

Provide `View document` where the current encrypted Vault file exists.

Rules:

```text
decrypt only inside the trusted Rust/Tauri boundary after Vault unlock
render requested PDF pages or validated PNG/JPEG images into memory for the in-app viewer
send rendered pixels, not raw PDF or image bytes, to the web UI
for PNG/JPEG evidence, apply ImageIO orientation and return only one re-encoded PNG bounded to 1,200 by 1,600 pixels and 1.92 million pixels; reject sources above 12,000 pixels in either dimension or 64 million pixels total
for CSV evidence, send at most the first 200 lines and 32 KiB of UTF-8 preview plaintext to the web UI
a CSV file that fits within both caps may appear in full, but the renderer never receives an unbounded or raw original-file byte payload
do not create a plaintext temporary file for normal viewing
release plaintext/page buffers on viewer close and Vault lock as far as the platform permits
never upload the file to a server merely for preview
show File deleted or Missing distinctly when the current file is unavailable
```

A truncated preview states that content is incomplete, reports how many source lines contributed to the preview without implying that the final line is complete, and points to `Save a copy` for the full file.

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

For canonical email-message evidence, label the equivalent action `Delete email evidence`; it follows the same tombstone and ledger-preservation rules without pretending that the user imported a file.

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

An explicit exact-hash Add/Restore reuses the tombstone and restores its current encrypted artifact after user confirmation. Automatic `Inbox` and Gmail scans treat the tombstone as a suppression record and do not restore it merely because the external file/message remains present. A byte-different file with the same semantic statement identity remains separate evidence under that statement identity.

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

When a transaction notification and statement row resolve to the same canonical event, Activity shows one amount with compact `Email` and `Statement` provenance. Both evidence items remain independently viewable; neither is discarded as a duplicate file.

## Statement coverage and missing-period prompts

Missing-statement detection is deterministic and provider-aware, not an AI guess. Derive it from accepted statement document type/period data plus the provider package's declared cadence and grace period.

Coverage is evaluated per resolved Money Source, child account/container, and statement document type. One multi-account statement may satisfy the period for each child account it actually identifies; unrelated transaction emails or other document types do not.

```text
accepted periods on both sides of a gap -> confirmed missing period
declared monthly cadence + established history + grace elapsed -> likely missing period
locked, unreadable, or parse-failed file for the period -> Needs attention, not missing
transaction-notification email -> does not satisfy statement coverage
on-demand export source such as Wise -> no monthly prompt unless its provider configuration declares one
```

The first UI is one calm Command Center card such as `DBS June statement may be missing`, with `Add file`, `Not expected`, and `Remind later`. Source detail may show the same coverage timeline. Derive expected periods from existing evidence rather than pre-creating expected-month rows. When these actions ship, persist only the explicit exception/snooze decision in the smallest owning-slice storage; do not build a general reminder engine.

## Empty states

Documents subview empty states should be source-specific.

Examples:

```text
No DBS statements yet. Add a file, set up CanCan Inbox, or connect Gmail.
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

The same summary covers message evidence using a human label such as `DBS transaction email`; it does not expose a synthetic filename or pipeline type.

Exact duplicates do not create duplicate records. Explicitly adding an exact file whose current Vault copy was deleted can restore that source document after confirmation; automatic scans skip it. A semantically matching file with different bytes remains available as separate additional evidence for the same statement identity.

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
- Add, drag/drop, Open With, watched-folder, and Gmail evidence converge on one capture/classification/parser flow.
- Only the selected root's `Inbox` child is rescanned deterministically, never modified by CanCan, and clearly remains outside the encrypted Vault; `Backups` is never an ingestion source.
- Automatic rescans respect deleted-evidence tombstones; only explicit user intent restores them.
- Unassigned evidence is handled through Command Center attention rather than a new top-level library.
- Transaction email and statement evidence may fold into one Activity item while both source records remain intact.
- Missing-period prompts use provider cadence and accepted statement periods; transaction emails and failed/locked files cannot falsely satisfy or erase coverage.
- A future iOS Share Extension remains a thin native intake target gated by transport, encryption, lifecycle, and release evidence.
