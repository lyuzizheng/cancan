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
- The official Phase 1 phone-to-Mac path is a Share-sheet Shortcut that saves into the user-authorized iCloud Drive `Cancan/Inbox`. CanCan does not scan all of iCloud Drive or Downloads, or move/delete source files.
- Local Inbox discovery is metadata-assisted but hash-authoritative: startup scans use creation/change/modification metadata to avoid reopening unchanged entries, while exact SHA-256 identity remains the deduplication boundary.
- An unclassified encrypted PDF may receive one privileged-host pass over all distinct saved statement passwords; decryption never identifies its Money Source.
- Existing Money Sources route automatically. A newly detected supported Money Source and its first account require one lightweight confirmation before creation.
- Exact file duplicates stop before parsing. Same-content statements stop before AI normalization only when a prior successful trusted parse has reusable records; otherwise normal parsing continues. Revised statements preserve evidence and version changed records.
- Statement coverage is a separate future AI-assisted source-analysis feature, not file, record, or relationship deduplication and not part of Local Inbox completion.
- A native iOS Share Extension is considered only if the Shortcut path fails its experience gate; it remains a thin intake companion, not a mobile ledger.

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

Global `Add` actions, drag/drop, and Open With may accept evidence from anywhere in the app. Evidence that is still unassigned projects as one Tasks `Needs action` row until classification, a compact source choice, or `Keep unassigned` resolves the active prompt; parked evidence remains available from Tasks `Parked` and Source/Documents. This is not a second document library or task authority.

## Acquisition channels

All channels create the same `source_documents` evidence artifact and then run the same trusted classifier, account resolver, parser, and reconciliation policy.

MVP order:

```text
1. Add files, drag/drop, and Open With CanCan
2. optional user-authorized CanCan iCloud Drive root plus its phone Share-sheet Shortcut
3. AirDrop or any cloud-drive app, followed by Add/Open With or the Shortcut
4. optional user-authorized Gmail rules
```

Do not build separate Dropbox, OneDrive, iCloud Drive, or AirDrop connectors. The initial automatic-folder contract below is only for the user-authorized iCloud Drive `Cancan` root; it is not a whole-iCloud scan or a provider-generic setup surface.

### Phase 1 phone Share Shortcut

CanCan provides one explicit install/test action for a versioned Shortcut that is available in the iPhone/iPad Share sheet. The user installs or updates it deliberately; the desktop app does not silently create, replace, or edit personal automations.

The Shortcut flow is one step after the system Share sheet:

```text
Share a supported statement
-> Run `Save to CanCan Inbox`
-> copy the selected file into `Cancan/Inbox`
-> show `Saved to CanCan Inbox. CanCan will process it on your Mac.`
```

The Shortcut does not ask for Money Source or account, does not inspect PDF contents, and contains no Vault key, statement password, parser, AI, dedupe, or ledger logic. It may validate only supported input types and whether the destination can be reached. Classification and every financial decision occur on the Mac.

Setup includes one safe synthetic test file or equivalent non-sensitive test action. Success proves only that the file reached `Inbox`; it does not claim that the Mac parsed a real statement. iCloud Shortcuts sync may make the installed Shortcut available on the user's other Apple devices, but CanCan must still show device-local install/test state truthfully rather than claiming every phone is configured.

#### Phase 1 Shortcut artifact and host contract

The Shortcut is versioned in the repository: `resources/shortcuts/` holds the artifact in Apple's pre-signing plist format, its manifest, and the install and real-device test path; `scripts/shortcuts/build-save-to-cancan-inbox.py` is its only writer, and `.agents/scripts/check-shortcut-artifact.sh` fails when either committed file drifts from it. The desktop host embeds both and installs that artifact.

The Share sheet's Save File action writes inside the Shortcuts container, so the accepted destination is `/Cancan/Inbox/` — `iCloud Drive ▸ Shortcuts ▸ Cancan ▸ Inbox` — and setup authorizes `iCloud Drive ▸ Shortcuts ▸ Cancan` as the CanCan root whose `Inbox` and `Backups` children CanCan owns. Confirming that landing folder on real hardware stays open until the owner's device runs the walkthrough; an observed different path replaces the generator constant and the documents that state it.

```text
phone_shortcut_status      the artifact version this build installs and the last
                           destination check; needs neither the Vault nor an
                           unlocked Inbox
install_phone_shortcut     writes the embedded artifact into the app's data
                           directory, signs it with the local Shortcuts CLI, and
                           hands it to Shortcuts; an artifact that could not be
                           signed is opened unsigned
test_phone_shortcut_inbox  the synthetic test action: under the Inbox scan guard,
                           create `CanCan Inbox check.txt` in the authorized
                           Inbox, read it back byte for byte, and remove it; an
                           existing file of that name is reported, never touched
```

The check's outcome is device-local state recorded beside the Vault per artifact version: a record written for another version reads back as `never_checked`, so a new artifact never inherits a pass proven with an older one. The check adds no capture path and no parser input, never changes a source file, and a capture that follows it takes the same shared pipeline as every other channel.

### Future macOS Finder Share intake

Phase 2 adds a native macOS `Share > CanCan` entry for supported files selected in Finder. This is a convenience intake adapter over the same host-owned Add/capture contract, not a new evidence pipeline, parser, source type, or capability registry.

The extension/service passes user-selected supported files to the running or launched CanCan app. When the CanCan process is not running or the Vault is manually locked, it copies those files into one bounded App Group handoff area and the main app imports them after unlock. A closed window alone is not a locked Vault. This is temporary intake staging, not a second source registry, parser, job engine, task authority, or long-lived inbox. CanCan still owns encrypted Vault capture, exact-hash dedupe, source registration, queued `parse_document`, and the derived Tasks `In progress`/`Needs action`/`Recently completed` projection. The extension must never receive the Vault key, statement passwords, database access, or parser logic.

The dedicated Phase 2 slice must choose and prove at-rest protection for the staged plaintext, private App Group permissions, filename/metadata exposure, exclusion from backup/indexing, atomic handoff, deletion after successful capture, cancellation cleanup, expiry cleanup after a bounded time, crash recovery, multi-file limits, native target/entitlement ownership, and Developer ID packaging. Do not implement plaintext staging without that security/lifecycle evidence.

### User-authorized CanCan iCloud Drive root

Through the system folder picker, the user explicitly authorizes one iCloud Drive `Cancan` root. CanCan's only app-owned child names below that root are:

```text
Inbox
Backups
```

Existing directories with those names are reused; missing directories are created. If either name is occupied by a non-directory, setup fails with an actionable state. CanCan never overwrites, moves, or deletes user data to resolve that conflict.

The user puts supported statements only in `Inbox`. Future local automation observes only that child, never the root itself or `Backups`. `Backups` is reserved for CanCan backup outputs and is never an ingestion source. Creating `Backups` means only that the target is prepared; it does not enable, schedule, or prove a backup. Backup bundle, scheduling, and restore behavior remain owned by [0009](./0009-backup-restore-versioning.md) and the blocked `backup-release` slice.

The user may later change or disable the root. This contract does not invent migration or cleanup behavior, and CanCan never deletes old roots or their child directories.

The root and its children remain outside the encrypted Vault and follow iCloud Drive's privacy and security model. The readiness evidence selected the native preflight-before-Rust-open rule used by the production host boundary below. Customer-facing folder status, watcher feedback, and setup controls remain renderer work.

Production root authorization is host-owned. The system folder picker produces one macOS security-scoped bookmark stored with the enabled state in device-local Keychain storage, outside the Vault database and backup bundle. The renderer cannot submit a path/bookmark or receive the full selected path. On process start after Vault unlock the host resolves and starts access; only manual Vault lock, process exit, or disabling the Inbox stops access. Closing the window, macOS session lock, and sleep do not discard authorization from a process that remains alive; sleep pauses observation until wake. A stale or denied bookmark produces an actionable reauthorization state and never falls back to scanning another location. Restore onto any device starts with local Inbox disabled until the user authorizes a root there.

The root gives a simple phone flow:

```text
bank app Share
-> Save to CanCan Inbox Shortcut
-> Shortcut copies to Cancan/Inbox and shows a saved receipt
-> Mac sync provider makes the file readable
-> CanCan captures it into the encrypted Vault
```

Rules:

```text
observe only the authorized root's Inbox child
scan on enable, process startup/unlock, wake recovery, manual refresh, and filesystem-change hints while the app runs
on startup/unlock enumerate every direct child and treat an unseen entry or changed creation/change/modification time, file identity, or size as a scan candidate
persist one minimum safe observation per direct child/file identity: creation/change/modification time, size, file identity, and last observed entry identity; never use one directory-wide high-water timestamp as proof that nothing changed
accept only supported regular PDF/CSV/image files that can be opened read-only
refuse a source file larger than 128 MB before reading it, on both acquisition channels, and report `source_file_too_large` (`Choose a file under 128 MB.`) instead of a generic import failure
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

Filesystem notifications are a wake-up hint, not the source of truth; deterministic `Inbox` rescans provide recovery after sleep, app exit, or sync delay. Creation, change, and modification timestamps are candidate-selection hints because iCloud or cross-filesystem copies may preserve or rewrite them. The settled bytes' SHA-256 remains the exact identity and deduplication authority. Do not watch the whole Downloads folder by default.

The two-second settle interval is a capture-protocol constant, not a user setting. The completed [local-inbox readiness evidence](../../spikes/local-inbox-readiness/EVIDENCE.md) proves this protocol against an ordinary local folder and the supported `Cancan/Inbox` iCloud Drive path, including a continuously changing file that never reaches capture early, a native placeholder preflight before Rust access, and a fresh-process restart lifecycle. Production folder automation remains a later slice.

### Optional email send-to-self

A phone user may Share to Mail and send a PDF/CSV to their own authorized Gmail account. The generic Gmail Inbox rule captures the attachment and trusted classification routes it. CanCan does not operate an inbound email service in MVP.

### Conditional future phone Share Extension

A native `Share to CanCan` extension is not the default roadmap commitment. First validate the Phase 1 Shortcut on real bank-app share sheets, multi-file inputs, Files/iCloud availability, install/update, and error recovery. Only if that experience is materially inadequate should a disposable native feasibility slice evaluate an iOS containing app plus thin Swift Share Extension, never a second finance UI.

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
Needs action
Processing
File deleted
Missing
```

How they are derived:

```text
Ready             document has usable metadata/records and no blocking issue
Needs action      locked PDF, parse failed, mapping needed, or review action exists
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

## Classification and first-source confirmation

Capture never asks the user to guess a source from a filename. Once trusted extraction/classification has enough evidence:

```text
exactly one existing Money Source/account matches -> route automatically
supported source matches but does not exist       -> create one confirmation item
multiple plausible existing sources/accounts      -> ask one compact choice
unsupported or still ambiguous                    -> keep parked; expose through Tasks `Parked` and Source/Documents
```

The first-source confirmation projects as one Tasks `Needs action` row whose primary click opens the exact focused Source-confirmation surface; it is not an inline duplicate card or blocking import modal. It shows only currently proven facts:

```text
safe file label and captured status
detected institution/source and document type
masked account identity and statement period when grounded
whether this is new, same content, or a possible revision
record summary when already available: new, existing, changed, and needs review
why confirmation is required
```

Primary action is `Create source and continue`. Secondary actions are `Choose an existing source`, `View document`, and `Keep unassigned`. The last action persists the versioned source-candidate decision, removes it from the actionable count, and keeps all evidence currently linked to that candidate under Tasks `Parked` and Source/Documents. The user can reopen the candidate from `Parked` and correct it before creation. CanCan must not create a Money Source or account from a filename/password guess, and must not require the user to review technical parser output.

For a protected unclassified PDF, the privileged host tries each distinct saved statement password once. If none works, the document projects into Tasks `Needs action` as `Password needed`; the owning password surface offers `Enter password` and `Leave parked`. Parking suppresses only that current password-required reason and preserves the evidence. A successful password only decrypts the file. After the source is classified and confirmed, CanCan may offer `Save for this source`; saving is never required to finish the current parse.

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
keep at most one decrypted document per Vault session so page paging and CSV preview do not re-read and re-decrypt the stored file; the renderer's viewer-close signal drops that buffer, and it is also dropped on Vault lock, when the document is deleted, and when another document is viewed
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

Use short human labels. Avoid exposing pipeline names such as `parse_document` in the product UI.

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

Statement coverage is a separate future feature that asks whether a source/account appears to be missing an expected statement period. It is not exact-file deduplication, stable-record identity/versioning, or reconciliation/linking.

The capability-specific API, allowed caller/data boundary, and shared renderer/app-internal/external AI architecture belong to the unified Phase 2 backend interface in `0019-ai-capability-platform-and-cli.md`. This feature must use that registry instead of defining a renderer-only command or generic source-data endpoint. One call covers one Money Source, child account, and statement document type. It starts with bounded structured period/processing data and record summaries, then may make at most one follow-up snippet request covering no more than three explicitly referenced in-scope documents, two pages per document, and 8,000 extracted characters total under the configured AI-data authorization. It never receives paths, arbitrary documents, unrestricted source bytes, or a whole-Vault dump. Model retention/cost and explanation/confidence rules remain unresolved.

Recurring evaluation runs through the existing durable local job system as an internal scheduled job. Accepted source/account/statement-period or processing-state changes mark the affected scope due. CanCan does not add a background daemon: routine work runs while the Desktop app is available, and startup/unlock evaluates a due scope only when no successful evaluation exists for the current local calendar day. Routine executions are not a normal user-facing Job card; users see only a useful missing-period suggestion or an actionable capability failure.

The first accepted actions are:

```text
No statement this period -> suppress only this exact Money Source, child account, document type, and period
Remind later             -> hide until a validated user-selected future date, then evaluate the period again
```

The host validates the current proposal and future reminder date and persists each decision idempotently. `No statement this period` never suppresses another period and does not permanently mark a provider/account as not expected. A coverage proposal or action cannot create, merge, link, commit, delete, or permanently ignore a financial record.

The pre-hardening PR41 checkpoint contained a deterministic monthly evaluator, direct Tauri commands/cards/actions, and exact-period decision persistence, but no AI capability or scheduled job. The post-PR41 hardening slice removes that evaluator and all production decision/validation code together with its direct renderer/command surface and provisional authority tests.

Current post-hardening behavior retains only Phase 2-compatible storage foundations: queryable statement type/period fields, source-document/account associations, and the exact-period decision table schema. No current production path reads or writes that decision table. Phase 2 rebuilds analysis, validation, and idempotent actions through `analyse_statement_coverage` on the unified backend interface. Do not rename the provisional `Not expected` action in place and imply that the separate coverage feature is complete.

## `local-inbox-backend` production boundary

The production backend starts after the review/ledger host boundary is complete and reuses the same Vault/source-document, parse, reconcile, review, job, and audit paths as Add/Open With. It does not wait for renderer craft and does not create a folder-specific ingestion pipeline. The customer-facing local-Inbox slice still completes only after both this backend and `review-ledger-ui` are integrated.

### Presentation-safe host contract

Freeze these command purposes before Kimi integration:

```text
root/status:
  choose the local Inbox root through the host picker
  get sanitized local Inbox status
  disable local Inbox
  request a fresh rescan

results:
  get the latest safe scan summary
```

Status may return configured/enabled/access state, safe child-folder labels, last scan time, counts by outcome, current attention actions, and wording that distinguishes `Backups folder prepared` from `backup configured`. It must not return paths, bookmarks, hashes, source bytes, or sensitive filenames in logs/errors. The explicit import summary may return bounded display filenames through the existing renderer evidence contract.

### `local-inbox-backend` implementation checkpoints

1. Add the smallest device-local Keychain value for the bookmark/enabled state and implement picker, bookmark resolution, child validation/creation, unlock/start, manual-lock/disable, reauthorization, and sanitized status.
2. Implement one scanner using the accepted native-preflight/two-second protocol above. Reuse the manual-import capture service; stable capture enqueues the one coarse `parse_document` job, followed by `reconcile_document`, under `0015-job-engine-error-model.md`.
3. Keep trusted statement document type/period columns available for later purpose-specific source-analysis APIs without adding coverage policy or expected-month rows to Local Inbox.

Backend completion requires persisted authorize -> restart -> scan -> job chain -> Review/Activity behavior, lock/disable recovery, migration tests, and local native/desktop gates. Statement coverage has its own later slice.

### `local-inbox-automation` integration checkpoint

Kimi integrates the authorize/status/refresh/import-summary/attention renderer against the frozen host contract. Completion requires deterministic host fixtures, browser behavior, accessibility, reduced motion, and final designer-level visual review.

Tests must cover existing/missing/conflicting child names, stale/denied bookmark, no path leakage, ordinary-folder scans with fake native preflight, changing/symlink/placeholder deferral, metadata-assisted startup scan without timestamp-only misses, filesystem-change wake-up, wake and restart rescan, exact and same-content duplicates, tombstone suppression, no source mutation, window-close continuation, manual-lock pause/unlock resume, expired-job recovery, and parse-history preservation. The opt-in real-iCloud run stays local and is not required CI.

## Empty states

Documents subview empty states should be source-specific.

Examples:

```text
No DBS statements yet. Add a file, set up CanCan Inbox, or connect Gmail.
No Wise exports yet. Import a CSV/PDF export to start.
This source has documents, but none match the current filter.
```

## Tasks intake projection, duplicate short-circuiting, and revisions

Every channel contributes through the acquisition receipt contract in `0002`; Intake does not create a separate queue or `Latest intake` module. Batch boundaries are deterministic:

```text
Add / drag-drop / Open With / Finder handoff
  -> enumerate one input set, then atomically create and seal one batch with pending items

Local Inbox, including files saved by the phone Shortcut
  -> after preflight/stability, atomically create and seal one batch for the eligible non-empty set

Gmail, only when its owning slice is enabled
  -> one batch per mailbox sync invocation that attempts at least one newly matched message/attachment

zero registered items
  -> no batch and no Tasks receipt
```

The host enumerates the eligible set first, then creates the batch and all ordered `pending` items and seals them in one transaction before capture. Later invocations cannot append. Every item then finalizes once; its source-document/audit registration and terminal receipt commit atomically. Placeholders, changing files, and transiently unreadable automatic entries remain deferred scanner candidates and are not included in that set. On restart, pending explicit-handoff items become actionable `Import interrupted` receipts because no source path/authority was persisted; pending automatic items become silent `discovery_retry` suppression receipts while the scanner retries the authoritative entry in a later pass/batch. The phone Shortcut's own receipt means only that iCloud accepted the save; the later Mac scan owns the CanCan intake batch.

Implementation keeps the authority boundary reviewable through bounded checkpoints:

1. Persist the receipt, candidate, and owner-scoped park state, then add transaction-scoped repository operations and route production capture producers through them.
2. Build one presentation-safe host-derived Tasks read model over the existing receipt, document, parse/job, source-confirmation, Review, setup, and audit owners. Use that same projection to reconcile sealed-batch completion and decide notification state; do not infer readiness from job status alone.
3. Freeze generated presentation types before renderer integration. The renderer lists and routes derived rows but cannot submit a generic task mutation; every action continues through its owning password, source, document, Review, or setup command.
4. Add OS notification delivery only after batch completion and the Tasks destinations are stable. The phone Shortcut and later acquisition adapters reuse the same admission and projection boundaries rather than adding another receipt or task path.

A sealed batch completes exactly once when every item is either rejected, suppressed, already-present, or restore-confirmation-required, or its resolved document has no active required pipeline step and currently derives as ready or actionable. Actionable completion may still leave a Tasks `Needs action` row. Startup/unlock reconciliation completes any eligible sealed batch left unfinished by a crash. The same transition decides whether notification is suppressed or pending; pending delivery uses the stable batch-derived OS notification identifier defined by `0002`, so a crash retry replaces instead of duplicating the visible notification.

Each user-meaningful outcome then projects into the unified Tasks presentation owned by `0006`:

```text
In progress         Processing
Recently completed Ready
Recently completed Already in CanCan
Recently completed Same statement content
Recently completed Updated statement
Recently completed Processed — no new records
Recently completed Source file restored
Needs action        Password needed
Needs action        New source detected
Needs action        Needs review
Needs action        Restore source file?
Recently completed Source file left deleted
Recently completed File not added
Needs action        Inbox file couldn't be added
Needs action        Import interrupted
```

Rejection UX depends on where the user can recover:

```text
explicit visible handoff rejection
  -> Recently completed: File not added
  -> no actionable badge or system notification
  -> deep-link to the acquisition receipt detail with bounded reason and Add again

background Local Inbox rejection after stable eligibility
  -> Needs action: Inbox file couldn't be added
  -> deep-link to Sources > CanCan Inbox issue detail with Try again, Open CanCan Inbox, and Leave parked
  -> the acquisition item owns the stable rejection/park/resolution decision; no source document is invented

restart of pending explicit handoff
  -> Needs action: Import interrupted
  -> deep-link to the same receipt detail with Add again

deferred automatic discovery
  -> no item, task, badge, or notification until a later stable attempt reaches registration
```

Rejected rows expose only the bounded safe input label and stable plain-language reason. `Open CanCan Inbox` is a host action over the existing authorized bookmark; it does not return the path to the renderer. `Add again` and `Try again` create a replacement item linked to the rejected item; admitting the replacement atomically resolves the old active or parked row. Local Inbox also correlates attempts through the scanner owner's opaque entry key and accepted-snapshot version, never through a label or path. A changed snapshot for the same entry replaces the stale row, but if that new attempt fails its new rejection remains actionable; an unrelated entry never clears the old row.

Command Center shows at most five rows across all Tasks groups; the `Recently completed` group there shows only the latest completed intake-batch receipts, while the full Tasks `Recently completed` filter keeps user-meaningful outcomes for 168 hours. Exact duplicates use their `already_present` batch-item receipt linked to the existing document, so they remain visible without a duplicate source-document row or parse job. The same projection covers message evidence using a human label such as `DBS transaction email`; it does not expose a synthetic filename or pipeline type.

Processing stops at the earliest safe duplicate boundary:

```text
same SHA-256 bytes with a current artifact
  -> Already in CanCan
  -> no new source-document row, parse job, records, or notification

same SHA-256 as a tombstone
  -> explicit Add creates actionable Restore source file? receipt
  -> Restore source file updates the projection to Source file restored
  -> Leave deleted updates it to Source file left deleted
  -> automatic discovery remains suppressed

different bytes, same canonical locally extracted content/table fingerprint
  -> Same statement content
  -> when the existing match has a successful trusted parse and reusable records,
     retain the captured artifact as additional evidence under that statement
     and do not call AI normalization or regenerate records
  -> otherwise continue normal parsing rather than inheriting an incomplete result

same trusted source/account/period, but financial content differs
  -> Updated statement
  -> retain both artifacts, parse the new revision, reuse/version stable records,
     and send changed or conflicting financial facts to Review

parse completes but every stable external record already exists
  -> Processed — no new records
  -> preserve parse evidence/history without duplicate ledger records
```

The content fingerprint is deterministic, versioned, and derived only from locally extracted canonical text/table observations. It is not a provider/source classifier and cannot by itself auto-commit, delete evidence, or overwrite prior facts. If local extraction is insufficient to prove equality, continue through normal classification/parsing rather than guessing.

Explicitly adding an exact file whose current Vault copy was deleted can restore that source document after confirmation; automatic scans skip it. A semantically matching file with different bytes remains available as separate evidence for the same statement identity.

For explicit Add/Open With, the picker or handoff closes as soon as encrypted capture and the atomic source-registration/`parse_document` transaction are durable. The document immediately projects into Tasks `In progress` and its normal Source/Documents surface as `Processing`; CanCan does not hold the Add modal open or send the user to a technical job page. Completion updates that row in place to `Recently completed` or `Needs action`. Password and source-confirmation rows expose `Leave parked` or `Keep unassigned`; choosing either removes the active count/Command Center row while preserving the evidence under Tasks `Parked` and Source/Documents.

Every task row deep-links to the exact document, password prompt, source confirmation, or statement-scoped Review group. Review remains the financial-decision authority. Returning restores the full Tasks filter and scroll context; intake does not copy Review, source, document, or job state into a generic task record.

## Background intake notifications

Settings exposes one `Background intake notifications` toggle, default Off. Ask for macOS notification permission only when the user turns it on.

Rules:

```text
visible CanCan window                 -> update Tasks; no system notification
no visible window, exact duplicates only
                                      -> stay silent
no visible window, new content ready  -> one redacted batch notification
no visible window, action required    -> one redacted batch notification
```

Aggregate one notification per completed intake batch instead of one per file or record. Notification text contains no provider/source, filename, account, amount, record detail, or financial content. Acceptable examples are `CanCan processed 3 files. 2 are ready and 1 needs action.` and `A statement needs action. Open CanCan to continue.`

Clicking a notification recreates the window. If the existing process still has the Vault key, route to the relevant Tasks row or exact owning action. If CanCan was manually locked or the process restarted, show `Unlock with Touch ID` or Vault password first, then resume the route. If macOS permission is denied, Settings shows `Notifications blocked by macOS` with `Open System Settings`; do not repeatedly prompt. Granular categories and quiet hours are outside MVP.

## Acceptance criteria

- Evidence is accessed from Source detail, not as a dominant standalone sidebar section.
- Document UI feels personal, polished, and concise.
- An always-visible embedded PDF preview is not required, but `View document` opens the available source only in CanCan's memory-backed viewer, and closing that viewer signals the host to release the cached decryption for the document.
- Normal viewing creates no plaintext temporary file and does not hand the original to an OS viewer.
- `Save a copy` is an explicit warned plaintext export to a user-selected location.
- Metadata appears above records in document detail.
- Technical extraction artifacts are hidden from normal UI.
- User-facing document states stay simple.
- `Delete source file` removes the current encrypted Vault file while retaining a navigable source-document tombstone and every record/ledger/audit relationship.
- There is no document Archive action in MVP; staged-record removal and committed-event Undo remain separate append-only actions.
- MVP search works through indexed structured fields.
- Future full-text search uses local SQLite FTS5, not a remote search service.
- The unified Tasks projection distinguishes processing, ready, exact duplicate, same content, updated statement, no-new-record, restored, password, new-source, and review outcomes without exposing technical job state or owning duplicate domain state.
- Exact duplicates stop before parse enqueue. Same-content evidence stops before AI normalization only when a prior successful trusted parse has reusable records; otherwise normal parsing continues. Revised statements retain/version evidence without duplicate ledger facts.
- Explicit Add reports success after encrypted Vault capture and the atomic source-registration/parse-job transaction are durable; asynchronous parsing reports its own later state.
- Add/Open With closes after durable capture, projects the document as Tasks `In progress`, updates the row in place, and uses a system notification only under the enabled redacted background-notification policy.
- Add, drag/drop, Open With, the phone Shortcut, watched-folder, and Gmail evidence converge on one capture/classification/parser flow.
- The Phase 1 Shortcut writes only to `Cancan/Inbox`, asks no source question, and provides a truthful saved receipt; a native iOS extension waits for evidence that the Shortcut experience is inadequate.
- Phase 2 Finder `Share > CanCan` is only another adapter over that same Add/capture flow. Its bounded App Group handoff for an exited/manually locked app owns no Vault, parser, source registry, or durable job queue and must pass the accepted temporary-plaintext protection/cleanup gates.
- Only the selected root's `Inbox` child is rescanned deterministically, never modified by CanCan, and clearly remains outside the encrypted Vault; `Backups` is never an ingestion source.
- Automatic rescans respect deleted-evidence tombstones; only explicit user intent restores them.
- Unassigned evidence is handled through Tasks `Needs action` or `Parked` rather than a new top-level library.
- Existing sources route automatically; a newly detected supported source receives one lightweight evidence-based confirmation before creation.
- Background notifications are off by default, permissioned only from Settings, batch-aggregated, redacted, and silent for exact-duplicate-only intake.
- Transaction email and statement evidence may fold into one Activity item while both source records remain intact.
- Statement coverage remains a separate AI-assisted source-analysis feature with no direct database/filesystem access or financial mutation authority.
- Coverage reuses the existing statement/source/account and exact-period decision storage, but replaces the provisional direct command/UI/monthly-heuristic authority with the unified Phase 2 capability and internal job.
- Its accepted period-scoped actions are `No statement this period` and `Remind later`; Local Inbox completion does not claim their production implementation.
- A future iOS Share Extension remains conditional on failed Shortcut experience evidence and is gated by transport, encryption, lifecycle, and release proof.
