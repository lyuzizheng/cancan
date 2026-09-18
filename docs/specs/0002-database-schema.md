# 0002. Database Schema Spec

## Goal

Design SQLite/SQLCipher schema for a local-first finance vault that supports file and message evidence, parsing, review, ledger, assets, positions, snapshots, and reconciliation.

## Validated storage boundary

SQLCipher plus FTS5 feasibility is verified by the [desktop spike](../../spikes/desktop-feasibility/EVIDENCE.md). The source-document/file lifecycle and version-1 encrypted envelope are accepted by the [backup/restore spec](0009-backup-restore-versioning.md) and [Vault security evidence](../../spikes/vault-security-validation/EVIDENCE.md). Keep cryptographic bytes out of ad hoc schema fields; the schema stores only the versioned wrapper/envelope metadata and file lifecycle needed by implemented queries.

## Database policy

- Use hand-written SQL migrations.
- Never edit a migration that has merged and may already be applied to an owner/dogfood Vault. Post-PR41 hardening appends a new versioned migration after `0007_local_inbox.sql`; it does not require resetting the owner's Vault.
- Do not use Prisma.
- Avoid clever, huge SQL queries that are hard to reason about.
- Prefer small indexed queries and TypeScript composition when it improves clarity.
- Use JSON fields for provider-specific metadata and evolving shapes to avoid premature table explosion.
- Add an index only for an implemented query, uniqueness rule, or idempotency rule.
- Every migration should be deterministic and reversible when practical.

## Slice-owned schema

The database grows with the implementation slices. Do not create the complete future schema in the first migration.

`synthetic-core-flow` creates only the tables needed to stage, validate, link, commit, and query synthetic records:

```text
money_sources
accounts
instruments
source_documents
parse_runs
external_records
ledger_events
ledger_legs
match_edges
review_items
audit_log
```

Later slices add their own tables only when their behavior is implemented. Examples include the production `source_documents` file columns, durable jobs, Gmail rules/sync state, and other connector state. A migration must name its owning slice and have an integration test that starts from a clean test database.

The ready `phase1-intake-experience` slice owns `intake_batches`, `intake_batch_items`, `money_source_candidates`, and the nullable source-document candidate/park fields defined below. Its forward database migration must upgrade a populated post-PR41 Vault; no earlier applied migration is rewritten. The recovery reminder instead extends the existing versioned device-local recovery configuration record owned by `0009`, not the SQL schema.

The Gmail slice has one `gmail_accounts` connection row per normalized mailbox address with recoverable connection status. OAuth refresh tokens stay in Keychain under stable mailbox-scoped secret keys; `0003-gmail-collector.md` owns the exact `pending_save`/`connected`/`pending_delete`/`disconnected` cross-store lifecycle. The later consent checkpoint adds a non-secret configured-provider fingerprint and separate attachment/body AI-consent timestamps when that behavior is implemented. Gmail search rules and sync cursors then reference the connection. A missing or changed provider recipient/disclosure fingerprint atomically clears both consent timestamps and disables Gmail-derived AI transfer until mailbox-level re-consent without disconnecting local Gmail capture. Disconnect deletes the token and disables rules while retaining the mailbox/rule/cursor/evidence identities for convergent reconnect. One Money Source may have rules in multiple mailboxes; mailbox/message identity, artifact SHA-256, and financial record identity remain separate idempotency layers.

## Ledger schema projection

`0013-ledger-assets-valuation.md` owns event and leg behavior. The schema must support it without duplicating that policy here:

```text
ledger_events distinguish posting from observation events
committed ledger events and legs are immutable
reversal events reference the event they reverse
replacement events may reference the reversal/correction chain
each accepted proposal version has a unique commit idempotency key
```

`0005-review-and-commit-policy.md` owns matching and audit behavior. The schema must support:

```text
many-to-many external-record/event links
an explicit match-edge role separating financial allocation from corroborating evidence
explicit allocation amount and unit for partial links
review status and unmatched remainder
append-only audit entries written atomically with financial mutations
```

Committed financial allocations remain immutable. A later evidence item may be attached to a committed event only through an append-only `corroborating_evidence` edge inserted with its audit entry in the same transaction. That edge has no allocation amount, never changes ledger legs or financial read models, and cannot be updated or deleted. The current synthetic-core trigger rejects every late edge; the Gmail slice must replace it with the narrower role-aware rule before promising statement-first late-email convergence.

`match_edges` is part of the synthetic core, not a deferred extension. A transfer can be represented as two source records linked through one canonical transfer event:

```text
external record from account A -> canonical transfer event <- external record from account B
```

The implemented query paths must support opening a source record and finding its canonical event plus sibling source records, and opening a canonical event and finding every linked source record.

## Account identity schema projection

`0014-money-overview-source-taxonomy.md` owns account identity and lifecycle behavior. The schema must support:

```text
candidate, confirmed, dismissed, archived, and merged account states
one stable provider account ID when the provider supplies it
one user-safe masked identifier for display when available
first-seen per-candidate accept/reject decisions before first commit
an audited rejected-proposal reference and restore transition for dismissed candidates
merged_into_account_id without rewriting committed ledger legs
idempotent account resolution and audited merge/archive transitions
```

Do not add a separate identifier table, keyed digests, strength levels, or key-version machinery until a supported provider demonstrates that the simpler account identity cannot represent its real inputs safely.

## Document and record identity

`0004-parser-contract.md` owns document/record identity and reparse behavior. The schema keeps its SHA-256 file identity, nullable versioned canonical-content fingerprint, semantic document identity, stable external-record key, and record version as separate fields.

In MVP, `source_documents` is the captured-evidence row and encrypted-artifact registry/tombstone. The name is retained to avoid a speculative rename: an artifact may be an imported file or a canonical message-evidence envelope created by a connector. Do not add a separate `vault_files`, inbox, or evidence-graph table.

One row represents one exact imported byte sequence or canonical envelope and stores the queryable identity and lifecycle needed by the product:

```text
file_sha256
canonical_content_fingerprint nullable until complete local extraction
canonical_content_fingerprint_version nullable with the fingerprint
canonical_content_match_document_id nullable; set when equal canonical content was matched to existing evidence
evidence_kind: file | email_message
original filename when present, MIME type, and byte size
encrypted Vault locator while the current file exists
file state: available, deleted, or missing
deletion timestamp and append-only audit reference when deleted
semantic document key used to group byte-different evidence for one statement identity
money_source_id nullable while trusted classification is pending
document type and statement-period bounds when classification provides them
```

`canonical_content_fingerprint`, `semantic_document_key`, and `money_source_id` are nullable while newly captured evidence is still unclassified or not completely extracted. Initial capture knows the exact artifact hash but must not accept later identity from the renderer or user. The trusted host writes the content fingerprint and its algorithm version only after complete local canonical extraction. It is intentionally non-unique because byte-different artifacts with equal content remain separate evidence rows. A picker, folder, Gmail rule, or future Share Extension may supply a source hint; it is not authority. The trusted classification and account-resolution path assigns the configured Money Source after provider verification and sets semantic identity after grounding the provider statement ID or accepted fallback inputs. Exact-hash deduplication remains available before classification; same-content short-circuiting begins only after a complete trusted fingerprint, and probable-statement grouping begins only after semantic identity exists.

An email without an attachment is not inserted directly as a ledger record. The Gmail connector first creates a bounded, deterministic UTF-8 message envelope containing only the provider fields approved by its rule, stores that encrypted envelope as `email_message` evidence, and sends it through the same parser contract. Its hash is computed over the canonical envelope bytes. The Gmail slice may add one minimal connector-idempotency mapping keyed by authenticated mailbox plus Gmail message ID; do not overload financial record identity or create a generic connector graph.

`document_type` and statement-period bounds are columns once the Source/Documents and coverage queries are implemented. They are not hidden in JSON because the product filters documents and derives missing-period prompts from them. Do not create expected-month rows or a reminder table until a persisted user decision or performance evidence requires one.

`deleted` means the user intentionally deleted the current encrypted Vault file. `missing` means the file should exist but storage cannot find or verify it. In both states, the `source_documents` row remains so external records, parse runs, review history, ledger navigation, and audit history do not break. A nullable locator or equivalent state projection must not erase the exact hash or evidence relationships.

Different byte sequences with the same canonical content or semantic statement identity remain separate `source_documents` rows. Equal content may reuse existing statement records without AI normalization; a changed statement revision is grouped by the semantic document key and versioned through normal parsing. The reused-record case is recorded durably: a matched artifact points at the document whose records it reuses through `canonical_content_match_document_id`, and that pointer - not the parse job's result payload - is the source of truth for the match. A self-referencing foreign key whose delete action clears the column keeps the link from outliving the evidence it joins: deleting the matched document's row clears the artifact's pointer, and deleting the artifact's row takes its own pointer with it. The matched document keeps its own records while the artifact keeps its own bytes, locator, and provenance. An explicit user import of an exact hash reuses the existing row and may offer to restore a deleted or missing current artifact instead of creating a duplicate row. Automatic folder/Gmail discovery treats a deleted tombstone as suppression and must not restore it; Gmail's mailbox/message mapping continues to point at that tombstone across envelope-version changes.

## Intake receipt and batch identity

The acquisition owner persists the minimum correlation needed for the latest-batch Tasks receipt and one-notification-per-batch policy. This is not a second Inbox, document registry, task authority, parser queue, or job system.

```text
intake_batches
  id
  acquisition_channel
  opened_at
  sealed_at
  completed_at nullable until every item reaches a derived terminal/actionable outcome
  notification_state: undecided | suppressed | pending | emitted
  notification_decided_at nullable
  notification_emitted_at nullable

intake_batch_items
  id
  intake_batch_id
  input_ordinal
  safe_input_label: bounded sanitized basename or generated label
  source_document_id nullable only when capture cannot produce or resolve a document
  capture_outcome: pending | captured | already_present | restore_confirmation_required | suppressed | rejected
  rejection_kind nullable: visible_receipt | background_action_required
  rejection_code nullable stable bounded code
  rejection_parked_at nullable
  rejection_resolved_at nullable
  rejection_resolution_kind nullable: superseded_by_new_attempt
  resolved_by_batch_item_id nullable self-reference
  acquisition_input_key nullable fixed-width channel-local opaque correlation token
  acquisition_input_version nullable fixed-width versioned digest of the accepted observation
  retry_of_batch_item_id nullable self-reference
  created_at
  finalized_at nullable while capture_outcome is pending
  UNIQUE(intake_batch_id, input_ordinal)
```

The host first enumerates the eligible input set, then creates one non-empty batch plus all ordered `pending` items and seals it in the same transaction. No persisted batch is unsealed and no later invocation may append items. A pending item has `created_at` and no `finalized_at`; capture moves it exactly once to a terminal capture outcome and sets `finalized_at`. Source-document/audit registration and the item transition commit atomically. A terminal item cannot return to pending, and the batch cannot complete while any item remains pending.

On restart, an explicit-handoff pending item becomes `rejected/background_action_required` with stable code `handoff_interrupted`, because no path or security-scoped input authority was persisted; its task asks the user to add the file again. An automatic-discovery pending item becomes `suppressed` with stable code `discovery_retry`, while the scanner's authoritative observation remains due and retries in a later pass/batch. These transitions are idempotent and never invent a captured document.

Terminal `capture_outcome` is an immutable acquisition receipt, not current parser/Review authority. Exact duplicates link the batch item to the existing `source_documents` row and retain `already_present`; they do not create another source document or job. `restore_confirmation_required` links to the tombstoned document and is actionable-terminal for batch completion; later `Restore source file` or `Leave deleted` is an audited document-owner decision, so the Tasks projection can move to `Source file restored` or `Source file left deleted` without rewriting the receipt. Active, actionable, ready, same-content, revision, and no-new-record presentation is derived by joining the item/document to the authoritative job, parse, source-confirmation, Review, record, and audit state.

Actionable acquisition rejections have acquisition-owned resolution without rewriting their immutable capture outcome. `Add again` or `Try again` creates a new item with `retry_of_batch_item_id`; admitting that replacement attempt atomically sets the prior rejection's `rejection_resolved_at`, `rejection_resolution_kind=superseded_by_new_attempt`, and `resolved_by_batch_item_id`. Automatic Local Inbox items also carry the existing scanner owner's fixed-width opaque `entry_key` and a fixed-width version-tagged digest of the canonical accepted file-snapshot fields as `acquisition_input_key` and `acquisition_input_version`; both are null for channels without automatic observation identity, and a database check requires them together. Neither value is a path or renderer-facing identity. Admitting a later attempt for the same entry key, whether the snapshot is unchanged or changed, similarly resolves the older active or parked rejection and makes only the new item current. A failed new attempt creates its own actionable rejection, so replacement never suppresses a genuinely new failure. A different entry key does not resolve the old row. These transitions are version-checked and idempotent; `safe_input_label` is presentation only and never participates in correlation.

`safe_input_label` is never supplied by the renderer as authority. For files it is only the final basename after removing path separators, control/bidirectional-control characters, and invalid text, truncated on a valid UTF-8 boundary to at most 240 bytes; an empty result becomes `File`. For message evidence it is a provider-approved generated label such as `Transaction email`, never a raw subject, sender, body, mailbox/message ID, or synthetic filename. It never stores a parent path, bookmark, locator, artifact hash, or secret, and notification text still omits it.

Automatic discovery includes an input in the atomically sealed pending set only after native preflight and the stable-capture protocol make it eligible for a durable capture attempt. A placeholder, still-changing, temporarily unreadable, or otherwise deferred entry creates no batch item; the scanner retains its normal observation/candidate state and retries on a later pass. Explicit user handoffs include every enumerated input so terminal rejection receives truthful feedback.

The host completes batches using the channel and terminal rules in `0017-evidence-documents-source-ux.md`. Completion is monotonic and restart-reconciled: a batch completes exactly once when no item is pending and each item is rejected, suppressed, already-present, or restore-confirmation-required, or its resolved document has no active required pipeline step and is ready or actionable. An actionable outcome is terminal for batch notification even though the user's task remains unresolved. Completion transactionally sets notification state to `suppressed` or `pending`. Delivery uses a stable OS notification identifier derived from the batch ID, then marks `emitted`; a crash retry with the same identifier replaces rather than duplicates the visible notification. An emitted or suppressed batch is never submitted again. A zero-item discovery/sync creates no batch.

These receipt rows may be deleted or compacted only after their 168-hour Tasks-presentation window and notification decision are no longer needed and no unresolved active/parked rejection task depends on them. A resolved/superseded rejection is no longer actionable or parked; its bounded receipt may age out normally. Deleting a receipt never deletes evidence, jobs, parse history, Review state, financial records, ledger events, or audit entries.

## Tasks projection persistence

The unified Tasks UI in `0006-command-center-ui.md` is a host-derived presentation projection, not a table or new workflow authority. Do not add `tasks`, `task_events`, `recent_tasks`, or a parallel status column that copies job, Review, source, document, or setup state. Active and recent rows are derived from their owning records and timestamps.

Only user decisions that must survive restart add owner-scoped state in the implementing slice:

```text
document attention park
  -> source_documents stores the parked attention reason/code and parked timestamp
  -> suppression applies only while the current derived reason still matches
  -> a different/new blocking reason becomes actionable instead of inheriting stale suppression

new-source confirmation
  -> money_source_candidates stores pending, kept_unassigned, or confirmed plus an expected version
  -> source_documents references the candidate while money_source_id remains null
  -> confirmed creation and routing remain one audited host transaction

ordinary setup reminder
  -> the versioned device-local recovery configuration record owned by `0009` stores nullable remind_after as the UTC due instant computed by `0006`
  -> no generic reminder table or calendar scheduler
```

Financial Review rows remain governed by `review_items`; Jobs remain governed by the durable job tables. Recently completed intake rows are derived from intake receipt, source-document, parse-run, job, and audit completion facts and disappear from Tasks presentation after 168 elapsed hours without deleting the underlying domain facts. A presentation row may carry a deterministic opaque row key and destination, but neither is financial or workflow identity.

`money_source_candidates` is the source-confirmation owner, not a task row:

```text
id
candidate_key_version
provider_key
candidate_scope_kind: provider_singleton | provider_root_id
candidate_scope_value: empty only for provider_singleton; otherwise the exact non-empty stable provider root ID, at most 512 UTF-8 bytes
status: pending | kept_unassigned | confirmed
version
confirmed_money_source_id nullable until confirmed
created_at
updated_at
UNIQUE(candidate_key_version, provider_key, candidate_scope_kind, candidate_scope_value)
```

The trusted classifier uses the versioned composite invariant directly; it does not concatenate identity strings. `provider_key` is the canonical package key. A stable provider root/source ID is treated as an opaque exact value with `provider_root_id`, without normalization or case folding; values beyond 512 UTF-8 bytes are rejected instead of truncated. When no stable ID exists, the empty value is valid only with `provider_singleton`; a database check enforces the tagged empty/non-empty invariant. Concurrent documents upsert and reference the same candidate. `Keep unassigned` version-checks and marks that candidate, leaving all linked evidence unassigned and parked. Confirmation version-checks the candidate, rechecks for a matching configured Money Source, creates at most one source if none exists, marks the candidate confirmed, routes every linked pending document, and appends its audit event in one transaction. A retry returns the already-confirmed source; a stale version reloads instead of creating another source. Filenames, passwords, masked labels, display names, delimiters, and naive concatenation never contribute to candidate identity.

Each external record stores the bounded original source row/object used for normalization and a validation summary:

```text
parse_runs store the complete normalization-profile and runtime/tool/model versions
external_records.posting_status stores provisional or posted when the source proves that distinction
external_records.raw_json stores one bounded source record, not full-document extraction
external_records.validation_json stores grounding and deterministic-validation outcomes
page, row, column, region, or provider location data may remain optional values inside raw_json
no separate evidence-reference table or location columns are required
date-only fields remain date-only rather than receiving an invented timezone
no generic assumptions_json column is added for speculative inference
```

The parser validates the raw record against job-scoped native/OCR/table observations before persistence. The raw record and validation summary are the durable audit context; optional location values are UI hints, not independently queried financial facts. Deleting the source file does not delete these bounded record-level facts. Full-document extraction is job-scoped and transient; it is not a durable database or search-index payload.

## Index policy

Create indexes only when they enforce an accepted invariant or serve a real repository query. The initial required access paths are:

```text
unique source_documents(file_sha256)
unique external_records(stable_record_key, version)
unique ledger_events(commit_idempotency_key)
an access path beginning with match_edges.external_record_id for record -> event/sibling navigation
an access path beginning with match_edges.ledger_event_id for event -> source-record navigation
```

The match-edge primary/unique key may satisfy one direction; do not create a redundant index for it. Account-provider identity receives a partial unique constraint only when a stable provider account ID is present on a non-merged account. `money_sources` receives a partial unique constraint on `(provider_key, provider_root_id)` only when the exact root identity is present, so one configured source exists per exact root identity per provider while singleton sources keep `provider_root_id` NULL. Status, date, list, and read-model indexes are added with the repository method that needs them and verified with its query plan or focused benchmark. Do not index optional values inside `raw_json` unless a later accepted feature introduces a real query for them.

## JSON field policy

Use JSON for bounded data that is displayed or validated as a whole:

```text
provider metadata
one raw source record/object per external record
validation details
AI explanation payloads
sync cursor state
source-specific account hints
```

Do not hide core query dimensions in JSON. If the UI filters by it often, make it a column.

## Benchmark policy

Benchmark implemented hot paths rather than a hypothetical complete schema. Each benchmark records the repository query, representative synthetic dataset, query plan, and threshold beside the feature that owns it. Start with record/event/sibling transfer navigation and add Command Center, Source detail, Review, asset, and event-list benchmarks only when those queries exist.

Do not freeze arbitrary global fixture cardinalities in this spec. Dataset size should be large enough to expose a regression in the owning query and may grow when real usage evidence justifies it.

## Test reset policy

Provide a deterministic test reset path:

```text
drop/recreate test database
run migrations
seed minimal sources/accounts
load fixtures
run parser/reconciliation tests
```

Acceptance requires integration tests to run from a clean database without manual setup.

## Acceptance criteria

- Schema changes use hand-written, versioned migrations.
- Applied/merged migrations remain immutable; hardening uses a forward migration and proves upgrade from the existing `0007_local_inbox.sql` state.
- Core query dimensions are columns rather than hidden in JSON.
- Exact source-file identity uses SHA-256 and remains separate from semantic document identity.
- A nullable versioned canonical-content fingerprint is written only after complete trusted local extraction, is not unique, and remains separate from both byte identity and semantic statement identity.
- `source_documents` covers imported files and canonical encrypted email-message envelopes without adding a second inbox/evidence table.
- Newly captured evidence may remain unassigned until trusted provider classification resolves one configured Money Source; channel and user source hints are not semantic authority.
- Queryable document type and statement-period bounds support Source/Documents filtering and a later bounded source-analysis API without expected-month rows.
- A newly captured file may keep semantic document identity null until trusted classification; renderer or user input cannot set it.
- `source_documents` is the MVP encrypted-file registry and tombstone; no separate `vault_files` table is required.
- Intake batch/item receipts atomically create one sealed pending set, use truthful create/finalize timestamps, provide bounded acquisition correlation and notification idempotency, resolve superseded rejection tasks without hiding a new failure, and reference the existing source document for exact duplicates without becoming an Inbox, task table, parser queue, or document authority.
- The Tasks UI is derived without a generic task/recent-task table; only owner-scoped park, source-confirmation, and setup-reminder decisions persist, and stale parking cannot suppress a different attention reason.
- Source confirmation has one checked, bounded, versioned composite candidate owner and an atomic recheck/create/route/audit transition, so delimiter/case ambiguity or concurrent documents cannot create duplicate Money Sources.
- Deleting or losing a current Vault file preserves its source-document row and every record, parse, review, ledger, and audit relationship.
- Byte-different equal-content or revised evidence uses separate source-document rows; equal content may skip AI/record regeneration, revisions group by semantic identity, explicit exact-hash re-import may restore a deleted current artifact, and automatic discovery respects the tombstone.
- Every external record retains one bounded validated raw source record plus a validation summary, without a separate field-evidence graph or permanent raw full-document text.
- Provisional transaction notifications remain queryable and cannot be mistaken for posted statement facts.
- Multiple Gmail mailbox connections keep OAuth state, provider-fingerprint-bound AI consent, tokens, rules, cursors, reconnect state, and message idempotency scoped to `gmail_account_id`; Money Sources may reference rules across those connections, and disconnect retains non-secret identities while deleting the Keychain token.
- Account identity supports a stable provider account ID when available, first-seen candidates, dismissed rejection/restore projections, archive, and merge redirects without using display names or masked suffixes as automatic identity.
- The schema supports immutable committed events, reversals, commit idempotency, many-to-many allocations, and atomic audit records.
- A late corroborating-evidence edge may be appended to a committed event only through the audited role-aware rule; committed financial allocations, legs, and prior edges remain immutable.
- Two source records can link through one canonical transfer event and are navigable in both directions.
- Indexes trace to an implemented query, uniqueness rule, or idempotency rule.
- Implemented hot query paths have focused benchmark/query-plan coverage with representative synthetic data.
- Integration tests can reset and migrate only a test database without manual setup.
