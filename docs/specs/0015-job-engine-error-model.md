# 0015. Job Engine and Error Model Spec

## Goal

Define CanCan's durable local job system, error model, recovery behavior, and user-facing blocked states.

The job engine exists to make long-running local-first finance workflows reliable and observable. It must not turn simple local functions into unnecessary micro-jobs.

## Implementation blocker

The source-file deletion and restore-to-new-path crash outcomes are accepted and validated by the [Vault security evidence](../../spikes/vault-security-validation/EVIDENCE.md). Production job wiring must still prove its durable idempotency key and cancellation boundaries in the owning implementation slice; the disposable spike did not model a persisted `jobs` row or user cancellation. Log/error redaction and crash-report consent remain open in the [active alignment register](../alignment-temp/alignment-progress.md). Backup scheduling, migration, and portability remain later `backup-release` concerns and do not block the source-deletion path in manual import.

## Stable decisions

- A durable job system is required from day 1.
- SQLite is the source of truth for job state.
- In-memory queues/workers are execution helpers only.
- Jobs should be coarse-grained and user-meaningful.
- Internal steps should be tracked inside a job when they do not need separate retry, user action, or independent audit.
- App startup must recover interrupted jobs.
- Password-protected PDFs use `blocked` state, not `failed`.
- Backup operations run through the same job engine.

## Coarse-grained job principle

Do not split every implementation function into a separate job.

Create a durable job only when at least one is true:

```text
it may run longer than a quick UI action
it touches external/local IO that can fail
it needs retry after app restart
it needs user-facing progress or history
it may become blocked on user action
it creates an audit-significant artifact
it needs cancellation or resume behavior
```

Otherwise, keep the work as an internal service function or a step inside an existing job.

## Recommended MVP job types

Use a small set of coarse jobs first:

```text
gmail_sync_rule
source_document_ingest
delete_source_file
parse_document
reconcile_document
commit_review_batch
backup_vault
restore_vault
```

### `gmail_sync_rule`

Runs one Gmail search rule.

Can internally perform:

```text
OAuth token refresh if needed
messages.list
messages.get
attachment metadata filtering
attachments.get
file hash dedupe
source_document creation
```

It should checkpoint cursor/history/processed message ids in `step_state_json` or sync-state tables so it can resume without duplicate imports.

Attachment and provider-approved transaction-message evidence both enter the existing source-document ingest/parse chain. Do not add one job type per email shape. Gmail overlap retries use mailbox/message identity, while artifact SHA-256 and financial record identity remain separate idempotency layers.

### `source_document_ingest`

Prepares a source document and initial source observations for parsing.

Can internally perform:

```text
file validation
mime detection
password-protected PDF detection
password unlock when a saved secret exists
native text extraction
structured table extraction
initial input-quality assessment
job-scoped extraction bundle creation
```

If the PDF is locked and no saved password works, the job becomes `blocked` with `blocked_reason = password_required`.

The saved password scope is the related Money Source. A failed saved password never loops blindly: the user chooses a session-only password or replaces that Money Source's saved Keychain secret.

The user-authorized CanCan root's `Inbox` child does not require a durable job merely to notice directory contents. Startup/unlock/manual scans and filesystem-change hints discover readable candidates; each candidate then enters the existing durable ingest job. A cloud placeholder, partial write, or unreadable file is deferred and retried by a later scan without modifying the source folder. A hash already represented by a user-deleted tombstone is a successful suppressed outcome, not a restore job; only explicit user intent starts restoration.

### `delete_source_file`

Deletes the current encrypted Vault file while preserving the source-document registry/tombstone and its relationships.

Can internally perform:

```text
persist the append-only deletion decision and audit reference
remove the current encrypted blob
project source_documents.file_state = deleted and clear its current locator
mark linked uncommitted records ineligible for automatic commit
retain parse, record, review, ledger, and audit navigation
```

The database transaction commits the append-only deletion decision, audit reference, tombstone projection, and auto-commit ineligibility before blob removal. Retry or startup recovery then converges as follows:

```text
before decision commit -> source and blob remain available
after decision commit  -> tombstone remains; recovery removes any blob
after blob removal     -> tombstone remains; recovery is a no-op
blob absent without a committed decision -> project Missing, not Deleted
```

The job never reverses a committed ledger event.

Before this path is considered implemented, integration tests must bind repeated attempts to one durable deletion intent, prove that no second decision is appended, and freeze the user-visible cancellation boundary. A later explicit exact-hash Add/Restore may restore the artifact as a new lifecycle transition; automatic discovery remains suppressed. Deleting that restored revision is a new intent.

### `parse_document`

Turns a source document plus job-scoped observations into a validated structured proposal and staged external records.

Can internally perform:

```text
provider/document skill selection
bounded AI normalization through single-pass or document-agent runtime
conditional OCR/page-region/tool requests
schema validation
evidence grounding
deterministic financial validation
parse_run creation
external_record staging
```

AI retries must create or preserve distinct parse run history. Do not silently overwrite previous model/prompt/input/output metadata.

The document agent receives only the current parse job and the fixed parser tools from `0004-parser-contract.md`. Step/submission budget exhaustion, invalid structured completion, and ungrounded required evidence become explicit parse outcomes rather than hidden retries.

### `reconcile_document`

Creates match candidates and review items from staged records.

Can internally perform:

```text
duplicate detection
transfer/card payment/top-up/FX candidate detection
confidence scoring
review item creation
safe auto-dedupe when policy allows
transaction-notification to posted-statement candidate detection
```

Ambiguous links should not auto-commit.

### `commit_review_batch`

Applies accepted review decisions or safe auto-commit policy.

Can internally perform:

```text
ledger_event creation
ledger_leg creation
match_edge updates
audit log writes
review item resolution
```

This job must be idempotent.

### `backup_vault`

Creates an encrypted backup bundle.

Can internally perform:

```text
SQLite checkpoint/snapshot
manifest generation
checksum generation
file collection
bundle encryption
copy to target folder
audit log write
```

Backup should support progress, cancel, retry, and clear error reporting.

### `restore_vault`

Validates and restores a backup into a new local Vault path before switching the active Vault atomically.

Can internally perform:

```text
manifest, checksum, encryption, and compatibility validation
restore into a new inactive path
open and validate the restored database and file inventory
atomically switch the active Vault only after every check passes
create a resumable Setup Checklist for excluded device secrets and configuration
```

Until the switch succeeds, the existing active Vault remains untouched. After switch, missing Gmail, AI, statement-password, and backup-folder setup blocks only dependent features; restored non-secret data remains readable.

Candidate files and their directory are synced and validated before writing and syncing a temporary locator. Atomic rename plus parent-directory sync selects the validated candidate, and startup recovery validates whichever locator is present and removes a stale temporary locator. A failed or invalid candidate never changes the active locator. The later `backup-release` implementation must additionally prove that retries address one durable candidate and freeze the user-visible cancellation boundary; those job semantics were not exercised by the disposable spike.

## Job status model

```text
queued
running
succeeded
failed
blocked
cancelled
```

Status meaning:

```text
queued      waiting for worker
running     leased by a worker
succeeded   completed successfully
failed      cannot continue without retry or fix
blocked     waiting for user action or missing prerequisite
cancelled   intentionally stopped
```

`blocked` is not failure. Examples:

```text
password_required
gmail_reconnect_required
ai_provider_required
user_review_required
backup_target_missing
```

## Minimal job table

```text
jobs
- id
- job_type
- status
- priority
- attempts
- max_attempts
- input_json
- step_state_json
- result_json
- error_json
- blocked_reason
- related_source_document_id
- related_money_source_id
- related_review_item_id
- lease_owner
- lease_until
- created_at
- updated_at
- started_at
- finished_at
```

Core query dimensions must be columns. Provider-specific payloads, checkpoints, and detailed errors can live in JSON.

## Lease and recovery

Workers claim jobs by setting `status = running`, `lease_owner`, and `lease_until`.

On app startup:

```text
find running jobs with expired lease_until
if idempotent/retryable -> move back to queued
if unsafe to retry blindly -> move to blocked or failed with recovery action
rebuild in-memory queue from SQLite
show compact status in Command Center
```

Do not assume the previous app session exited cleanly.

## Retry policy

Retry policy is job-type specific.

Recommended defaults:

```text
Gmail/network transient errors: auto retry with backoff
file IO/transient extraction errors: limited auto retry
AI parse transient/provider errors: retry allowed, preserve parse history
normalizer budget exhaustion or invalid structured completion: no blind retry, create review/error
evidence or financial validation failure: no blind retry, create review/error
password_required: blocked until user action
Gmail auth revoked: blocked until reconnect
backup target unavailable: blocked or failed with retry action
```

Retries must be idempotent. Re-running a job must not duplicate source documents, external records, ledger events, or backup bundles.

## Error model

Use two layers of error information.

### User-facing error

Short and actionable:

```text
Password needed
Gmail needs reconnect
Could not parse statement
Backup target unavailable
Review required before commit
```

### Technical error

Stored in `error_json` for debugging and future reports:

```text
error_code
message
provider
job_type
attempt
retryable
related ids
raw cause when safe
```

Parser-specific codes distinguish at least `normalizer_budget_exhausted`, `structured_proposal_invalid`, and `evidence_grounding_failed` from transient provider/network failures so the job engine does not spend money retrying deterministic rejection.

Do not put secrets, statement passwords, OAuth tokens, AI keys, or full sensitive document text in `error_json`.

## User-facing surfaces

### Command Center

Show compact status only:

```text
12 processed
3 need attention
1 failed
last Gmail scan at ...
last backup at ...
```

### Jobs page

Show advanced history and controls:

```text
status
job type
related source/document
progress/current step
retry/cancel action
error detail
created/started/finished timestamps
```

### Review/action items

Blocked jobs that require user action should create or link to a review/action item.

Examples:

```text
Unlock DBS statement PDF
Reconnect Gmail
Set up AI provider
Choose account mapping
Review possible card payment link
```

## Progress model

A job may expose progress through `step_state_json`.

Use coarse, user-understandable steps:

```text
searching Gmail
importing attachments
unlocking document
extracting text
reading statement
checking records
matching movements
creating backup
```

Avoid exposing internal implementation noise.

## Job chains, not full DAG in MVP

MVP should not implement a complex general-purpose DAG engine.

Use simple chaining:

```text
job.result_json.next_job_type
job.related_source_document_id
job.related_review_item_id
```

The service layer can enqueue the next coarse job after successful completion.

Full DAG features can be added later if needed, but the MVP should stay easy for one developer and AI coding agents to reason about.

## Tests / acceptance criteria

- Job state is persisted in SQLite.
- App startup recovers expired running jobs.
- Job retries are idempotent.
- Password-protected PDFs become blocked, not failed.
- Blocked jobs have user-facing action surfaces.
- Gmail sync reruns do not duplicate attachments or source documents.
- AI parse retries preserve parse history.
- Commit jobs do not duplicate ledger events when retried.
- Backup runs as a job and reports progress/errors.
- Source-file deletion converges idempotently on a tombstone, never breaks evidence navigation, and never changes committed ledger events.
- Restore validates a new inactive Vault before the atomic switch and exposes resumable setup for device-local secrets afterward.
- Command Center shows compact status; Jobs page shows details.
- Implementation avoids over-splitting simple internal work into tiny jobs.
