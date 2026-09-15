use crate::{
    local_inbox::FileSnapshot,
    vault::{FileVault, PreparedSource, StoredFile},
    viewer::validate_image_container,
};
use hkdf::Hkdf;
use rand::{RngCore, rngs::OsRng};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashSet},
    error::Error,
    fs, io,
    path::Path,
};
use zeroize::Zeroizing;

#[cfg(test)]
pub(crate) use accounts::{
    AccountConfirmationCandidate, AccountConfirmationStatus, CandidateAccountDecision,
};
pub(crate) use accounts::{
    AccountConfirmationOutcome, AccountConfirmationPrompt, CandidateAccountDecisionInput,
};
const KEY_LEN: usize = 32;
pub(crate) const DATABASE_FILE_NAME: &str = "finance.sqlite";
const DATABASE_KEY_CONTEXT: &[u8] = b"cancan:database:v1";

const REVIEW_POLICY_VERSION: &str = "review-ledger-v1";
const ACCOUNT_CONFIRMATION_POLICY_VERSION: &str = "account-confirmation-v1";
const COMMIT_REVIEW_BATCH_JOB_TYPE: &str = "commit_review_batch";
const PARSE_DOCUMENT_JOB_TYPE: &str = "parse_document";
const PARSE_DOCUMENT_LEASE_SECONDS: i64 = 300;
const RECONCILE_DOCUMENT_JOB_TYPE: &str = "reconcile_document";
const RECONCILE_DOCUMENT_LEASE_SECONDS: i64 = 300;
const COMMIT_REVIEW_BATCH_LEASE_SECONDS: i64 = 300;
const MAX_SUPPORTED_RELATIONSHIP_WINDOW_DAYS: i64 = 7;
const MAX_PERSISTED_PARSE_JSON_BYTES: usize = 16 * 1024;

type StoreResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParseDocumentJob {
    pub(crate) document_id: String,
    pub(crate) job_id: String,
    pub(crate) logical_run_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParseDocumentClaim {
    pub(crate) claim_token: String,
    pub(crate) document_id: String,
    pub(crate) job_id: String,
    pub(crate) logical_run_key: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParseDocumentJobInput {
    document_id: String,
    logical_run_key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDocumentImportStatus {
    Imported,
    AlreadyPresent,
    Restored,
    RestoreConfirmationRequired,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDocumentImportOutcome {
    pub document_id: String,
    pub status: SourceDocumentImportStatus,
    #[serde(skip)]
    pub(crate) intake_item_id: Option<String>,
}

#[derive(Debug)]
pub struct SourceDocumentImport<'a> {
    pub audit_actor: &'a str,
    pub audit_id: &'a str,
    pub audit_policy_version: &'a str,
    pub audit_reason: &'a str,
    pub document_id: &'a str,
    pub mime_type: &'a str,
    pub original_filename: &'a str,
    #[cfg(test)]
    pub source_path: &'a Path,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SourceDocumentView {
    pub byte_size: u64,
    pub document_id: String,
    pub encrypted_locator: Option<String>,
    pub file_sha256: String,
    pub file_state: String,
    pub mime_type: String,
    pub money_source_id: Option<String>,
    pub original_filename: String,
    pub received_at: String,
    pub semantic_document_key: Option<String>,
}

pub struct SourceDocumentFileInput {
    pub file_sha256: String,
    pub mime_type: String,
    pub plaintext: Zeroizing<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewItemSummary {
    pub(crate) account_label: String,
    pub(crate) amount_value: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) event_type: Option<String>,
    pub(crate) posted_on: Option<String>,
    pub(crate) reason_code: String,
    pub(crate) record_committed: bool,
    pub(crate) record_id: String,
    pub(crate) record_version: i64,
    pub(crate) review_item_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewItemDetail {
    pub(crate) account_label: String,
    pub(crate) amount_value: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) document_label: String,
    pub(crate) event_type: Option<String>,
    pub(crate) posted_on: Option<String>,
    pub(crate) reason_code: String,
    pub(crate) record_committed: bool,
    pub(crate) record_id: String,
    pub(crate) record_version: i64,
    pub(crate) review_item_id: String,
    pub(crate) source_label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecentActivitySummary {
    pub(crate) can_undo: bool,
    pub(crate) event_date: String,
    pub(crate) event_id: String,
    pub(crate) event_type: String,
    pub(crate) source_labels: Vec<String>,
    pub(crate) spending: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoneyOverviewAmount {
    pub(crate) account_id: String,
    pub(crate) account_label: String,
    pub(crate) as_of: String,
    pub(crate) currency: String,
    pub(crate) value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoneyOverview {
    pub(crate) assets: Vec<MoneyOverviewAmount>,
    pub(crate) liabilities: Vec<MoneyOverviewAmount>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelationshipCandidateSummary {
    pub(crate) account_label: String,
    pub(crate) amount_value: String,
    pub(crate) currency: String,
    pub(crate) event_type: String,
    pub(crate) posted_on: String,
    pub(crate) record_id: String,
    pub(crate) record_version: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewMutationStatus {
    Acknowledged,
    Conflict,
    RelationshipAccepted,
    Removed,
    Updated,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewMutationOutcome {
    pub(crate) reason: Option<&'static str>,
    pub(crate) record_version: Option<i64>,
    pub(crate) review_item_id: Option<String>,
    pub(crate) status: ReviewMutationStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewJobStatus {
    Blocked,
    Cancelled,
    Failed,
    Queued,
    Running,
    Succeeded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewBatchGroupStatus {
    AlreadyCommitted,
    Committed,
    Stale,
    StillNeedsReview,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewBatchGroupOutcome {
    pub(crate) reason: Option<String>,
    pub(crate) record_ids: Vec<String>,
    pub(crate) status: ReviewBatchGroupStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewJobSummary {
    pub(crate) created_at: String,
    pub(crate) finished_at: Option<String>,
    pub(crate) job_id: String,
    pub(crate) outcomes: Vec<ReviewBatchGroupOutcome>,
    pub(crate) status: ReviewJobStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CoreReviewRecord {
    pub(crate) account_balance_delta: String,
    pub(crate) account_id: String,
    pub(crate) account_type: String,
    pub(crate) currency: String,
    pub(crate) id: String,
    pub(crate) instrument_id: String,
    pub(crate) posted_on: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ReviewRelationshipCandidateInput {
    pub(crate) candidates: Vec<CoreReviewRecord>,
    pub(crate) event_type: String,
    pub(crate) record: CoreReviewRecord,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CoreReviewLeg {
    pub(crate) account_id: String,
    pub(crate) amount_value: String,
    pub(crate) currency: String,
    pub(crate) instrument_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CorePreparedReviewEvent {
    pub(crate) event_class: String,
    pub(crate) event_date: String,
    pub(crate) event_type: String,
    pub(crate) legs: Vec<CoreReviewLeg>,
    pub(crate) source_record_ids: Vec<String>,
    pub(crate) spending: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CorePreparedReversalEvent {
    pub(crate) event_class: String,
    pub(crate) event_date: String,
    pub(crate) event_type: String,
    pub(crate) legs: Vec<CoreReviewLeg>,
    pub(crate) spending: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UndoStatus {
    AlreadyUndone,
    Undone,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UndoOutcome {
    pub(crate) event_id: String,
    pub(crate) status: UndoStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitReviewBatchInput {
    review_item_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ClaimedReviewBatch {
    pub(crate) job_id: String,
    pub(crate) lease_owner: String,
    pub(crate) review_item_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct CommitReviewGroup {
    pub(crate) event_type: String,
    pub(crate) records: [CoreReviewRecord; 2],
    relationship_id: String,
    review_item_ids: [String; 2],
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct MoneySourceView {
    pub(crate) display_name: String,
    pub(crate) money_source_id: String,
    pub(crate) source_type: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatementPasswordStatus {
    PendingDelete,
    PendingSave,
    Saved,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StatementPasswordState {
    pub(crate) money_source_id: String,
    pub(crate) secret_storage_key: String,
    pub(crate) status: StatementPasswordStatus,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StatementPasswordSource {
    pub(crate) display_name: String,
    pub(crate) has_saved_password: bool,
    pub(crate) money_source_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDocumentRoutingStatus {
    Routed,
    NeedsAttention,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDocumentRoutingOutcome {
    pub account_ids: Vec<String>,
    pub document_id: String,
    pub money_source_id: Option<String>,
    pub reason: Option<&'static str>,
    pub status: SourceDocumentRoutingStatus,
}

impl SourceDocumentRoutingOutcome {
    pub(crate) fn needs_attention(document_id: &str, reason: &'static str) -> Self {
        Self {
            account_ids: Vec::new(),
            document_id: document_id.to_owned(),
            money_source_id: None,
            reason: Some(reason),
            status: SourceDocumentRoutingStatus::NeedsAttention,
        }
    }
}

#[derive(Debug)]
pub struct TrustedAccountCandidate<'a> {
    pub account_id: &'a str,
    pub account_type: &'a str,
    pub currency: Option<&'a str>,
    pub display_name: &'a str,
    pub masked_identifier: Option<&'a str>,
    pub provider_account_id: Option<&'a str>,
}

#[derive(Debug)]
pub struct TrustedDocumentClassification<'a> {
    pub accounts: &'a [TrustedAccountCandidate<'a>],
    pub audit_id: &'a str,
    pub document_id: &'a str,
    pub document_type: Option<&'a str>,
    pub provider_key: &'a str,
    pub provider_root_id: Option<&'a str>,
    pub semantic_document_key: &'a str,
    pub statement_period_from: Option<&'a str>,
    pub statement_period_to: Option<&'a str>,
}

#[derive(Clone, Debug)]
pub(crate) struct ValidatedExternalRecordInput {
    pub(crate) account_id: String,
    pub(crate) account_balance_delta: Option<String>,
    pub(crate) amount_value: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) event_type: Option<String>,
    pub(crate) posted_on: Option<String>,
    pub(crate) posting_status: Option<String>,
    pub(crate) raw_json: String,
    pub(crate) record_type: String,
    pub(crate) stable_record_key: String,
    pub(crate) validation_json: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ValidatedStructuredParseInput {
    pub(crate) normalization_profile_id: String,
    pub(crate) profile_json: String,
    pub(crate) records: Vec<ValidatedExternalRecordInput>,
}

#[derive(Debug)]
struct ExistingDocument {
    document_id: String,
    encrypted_locator: Option<String>,
    file_sha256: String,
    file_state: String,
}

pub struct ManualImportStore {
    connection: Connection,
    files: FileVault,
    master_key: Zeroizing<[u8; KEY_LEN]>,
}

impl ManualImportStore {
    pub fn open(root: &Path, master_key: Zeroizing<[u8; KEY_LEN]>) -> StoreResult<Self> {
        fs::create_dir_all(root)?;
        Self::open_with_flags(
            root,
            master_key,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
    }

    pub fn open_existing(root: &Path, master_key: Zeroizing<[u8; KEY_LEN]>) -> StoreResult<Self> {
        Self::open_with_flags(root, master_key, OpenFlags::SQLITE_OPEN_READ_WRITE)
    }

    fn open_with_flags(
        root: &Path,
        master_key: Zeroizing<[u8; KEY_LEN]>,
        flags: OpenFlags,
    ) -> StoreResult<Self> {
        let mut connection =
            open_encrypted_database(&root.join(DATABASE_FILE_NAME), &master_key, flags)?;
        apply_migrations(&mut connection)?;
        let mut store = Self {
            connection,
            files: FileVault::new(root),
            master_key,
        };
        store.reconcile_files()?;
        store.recover_interrupted_intake_items()?;
        store.recover_expired_parse_document_jobs()?;
        store.recover_interrupted_parse_document_jobs()?;
        store.recover_interrupted_review_jobs()?;
        store.recover_expired_review_jobs()?;
        Ok(store)
    }

    pub(crate) fn master_key(&self) -> &[u8; KEY_LEN] {
        &self.master_key
    }

    pub(crate) fn prepare_source_path(
        mime_type: &str,
        source_path: &Path,
    ) -> StoreResult<PreparedSource> {
        let source = FileVault::prepare(source_path)?;
        if matches!(mime_type, "image/png" | "image/jpeg") {
            validate_image_container(source.plaintext(), mime_type)?;
        }
        Ok(source)
    }

    pub(crate) fn prepare_source_bytes(
        mime_type: &str,
        captured_bytes: Zeroizing<Vec<u8>>,
    ) -> StoreResult<PreparedSource> {
        validate_captured_container(mime_type, &captured_bytes)?;
        let source = FileVault::prepare_bytes(captured_bytes)?;
        if matches!(mime_type, "image/png" | "image/jpeg") {
            validate_image_container(source.plaintext(), mime_type)?;
        }
        Ok(source)
    }

    pub(crate) fn source_capture_plan(
        &self,
        input: &SourceDocumentImport<'_>,
        source: &PreparedSource,
        restore_deleted_document_id: Option<&str>,
    ) -> StoreResult<SourceCapturePlan> {
        validate_import(input)?;
        let existing = find_exact_document(&self.connection, source.file_sha256())?;
        match (existing.as_ref(), restore_deleted_document_id) {
            (Some(existing), None) if existing.file_state == "deleted" => {
                return Ok(SourceCapturePlan::RestoreConfirmationRequired(
                    SourceDocumentImportOutcome {
                        document_id: existing.document_id.clone(),
                        status: SourceDocumentImportStatus::RestoreConfirmationRequired,
                        intake_item_id: None,
                    },
                ));
            }
            (Some(existing), Some(expected_document_id))
                if existing.file_state == "deleted"
                    && existing.document_id == expected_document_id => {}
            (_, Some(_)) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "selected source no longer matches the deleted document",
                )
                .into());
            }
            _ => {}
        }
        Ok(SourceCapturePlan::Capture(SourceCapture {
            files: self.files.clone(),
            master_key: self.master_key.clone(),
            replace_existing: existing
                .as_ref()
                .is_some_and(|document| document.file_state != "available"),
        }))
    }

    pub(crate) fn deleted_source_hashes(&self) -> StoreResult<BTreeSet<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT file_sha256 FROM source_documents WHERE file_state = 'deleted'")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub(crate) fn local_inbox_entry_is_current(
        &self,
        entry_key: &str,
        snapshot: &FileSnapshot,
    ) -> StoreResult<bool> {
        self.connection
            .query_row(
                "SELECT creation_nanoseconds, creation_seconds, change_nanoseconds, change_seconds, \
                        device, inode, last_observed_entry_identity, modified_nanoseconds, \
                        modified_seconds, size_bytes \
                 FROM local_inbox_entry_observations WHERE entry_key = ?1",
                [entry_key],
                |row| {
                    Ok(
                        row.get::<_, i64>(0)? == snapshot.creation_nanoseconds
                            && row.get::<_, i64>(1)? == snapshot.creation_seconds
                            && row.get::<_, i64>(2)? == snapshot.change_nanoseconds
                            && row.get::<_, i64>(3)? == snapshot.change_seconds
                            && row.get::<_, i64>(4)? == i64::try_from(snapshot.identity.device).unwrap_or(-1)
                            && row.get::<_, i64>(5)? == i64::try_from(snapshot.identity.inode).unwrap_or(-1)
                            && row.get::<_, String>(6)?
                                == format!("{}:{}", snapshot.identity.device, snapshot.identity.inode)
                            && row.get::<_, i64>(7)? == snapshot.modified_nanoseconds
                            && row.get::<_, i64>(8)? == snapshot.modified_seconds
                            && row.get::<_, i64>(9)? == i64::try_from(snapshot.size).unwrap_or(-1),
                    )
                },
            )
            .optional()
            .map(|current| current.unwrap_or(false))
            .map_err(Into::into)
    }

    pub(crate) fn record_local_inbox_entry_observation(
        &mut self,
        entry_key: &str,
        snapshot: &FileSnapshot,
    ) -> StoreResult<()> {
        let device = i64::try_from(snapshot.identity.device)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid file device"))?;
        let inode = i64::try_from(snapshot.identity.inode)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid file inode"))?;
        let size = i64::try_from(snapshot.size)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid file size"))?;
        self.connection.execute(
            "INSERT INTO local_inbox_entry_observations( \
               entry_key, creation_nanoseconds, creation_seconds, change_nanoseconds, change_seconds, \
               device, inode, last_observed_entry_identity, modified_nanoseconds, modified_seconds, size_bytes \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) \
             ON CONFLICT(entry_key) DO UPDATE SET \
               creation_nanoseconds = excluded.creation_nanoseconds, \
               creation_seconds = excluded.creation_seconds, \
               change_nanoseconds = excluded.change_nanoseconds, \
               change_seconds = excluded.change_seconds, device = excluded.device, inode = excluded.inode, \
               last_observed_entry_identity = excluded.last_observed_entry_identity, \
               modified_nanoseconds = excluded.modified_nanoseconds, \
               modified_seconds = excluded.modified_seconds, size_bytes = excluded.size_bytes, \
               updated_at = CURRENT_TIMESTAMP",
            params![
                entry_key,
                snapshot.creation_nanoseconds,
                snapshot.creation_seconds,
                snapshot.change_nanoseconds,
                snapshot.change_seconds,
                device,
                inode,
                format!("{}:{}", snapshot.identity.device, snapshot.identity.inode),
                snapshot.modified_nanoseconds,
                snapshot.modified_seconds,
                size,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn queued_parse_document_jobs(&mut self) -> StoreResult<Vec<ParseDocumentJob>> {
        self.recover_expired_parse_document_jobs()?;
        let mut statement = self.connection.prepare(
            "SELECT id, related_source_document_id, input_json FROM jobs \
             WHERE job_type = ?1 AND status = 'queued' ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([PARSE_DOCUMENT_JOB_TYPE], |row| {
            let input: ParseDocumentJobInput = serde_json::from_str(&row.get::<_, String>(2)?)
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok(ParseDocumentJob {
                document_id: row
                    .get::<_, Option<String>>(1)?
                    .unwrap_or(input.document_id),
                job_id: row.get(0)?,
                logical_run_key: input.logical_run_key,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub(crate) fn start_parse_document_job(
        &mut self,
        job: &ParseDocumentJob,
    ) -> StoreResult<Option<ParseDocumentClaim>> {
        let claim_token = new_database_id("parse-lease");
        let changed = self.connection.execute(
            "UPDATE jobs SET status = 'running', attempts = attempts + 1, \
                     lease_owner = ?1, lease_until = datetime('now', ?2), \
                     started_at = COALESCE(started_at, CURRENT_TIMESTAMP), updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?3 AND related_source_document_id = ?4 AND job_type = ?5 \
               AND status = 'queued' AND attempts < max_attempts",
            params![
                claim_token,
                format!("+{PARSE_DOCUMENT_LEASE_SECONDS} seconds"),
                job.job_id,
                job.document_id,
                PARSE_DOCUMENT_JOB_TYPE,
            ],
        )?;
        if changed == 1 {
            return Ok(Some(ParseDocumentClaim {
                claim_token,
                document_id: job.document_id.clone(),
                job_id: job.job_id.clone(),
                logical_run_key: job.logical_run_key.clone(),
            }));
        }
        self.connection.execute(
            "UPDATE jobs SET status = 'failed', blocked_reason = 'retry_limit_reached', \
                     finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?1 AND job_type = ?2 AND status = 'queued' AND attempts >= max_attempts",
            params![job.job_id, PARSE_DOCUMENT_JOB_TYPE],
        )?;
        Ok(None)
    }

    pub(crate) fn finish_parse_document_job(
        &mut self,
        claim: &ParseDocumentClaim,
        outcome: &SourceDocumentRoutingOutcome,
    ) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        match outcome.status {
            SourceDocumentRoutingStatus::Routed => {
                let changed = transaction.execute(
                    "UPDATE jobs SET status = 'succeeded', result_json = ?1, lease_owner = NULL, \
                         lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
                     WHERE id = ?2 AND related_source_document_id = ?3 AND job_type = ?4 \
                       AND status = 'running' AND lease_owner = ?5",
                    params![
                        serde_json::json!({ "nextJobType": RECONCILE_DOCUMENT_JOB_TYPE }).to_string(),
                        claim.job_id,
                        claim.document_id,
                        PARSE_DOCUMENT_JOB_TYPE,
                        claim.claim_token,
                    ],
                )?;
                if changed != 1 {
                    return Err(io::Error::other("parse job is no longer claimed").into());
                }
                enqueue_reconcile_document(&transaction, &claim.document_id)?;
            }
            SourceDocumentRoutingStatus::NeedsAttention => {
                let changed = transaction.execute(
                    "UPDATE jobs SET status = 'blocked', blocked_reason = ?1, lease_owner = NULL, \
                         lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
                     WHERE id = ?2 AND related_source_document_id = ?3 AND job_type = ?4 \
                       AND status = 'running' AND lease_owner = ?5",
                    params![
                        outcome.reason.unwrap_or("classification_uncertain"),
                        claim.job_id,
                        claim.document_id,
                        PARSE_DOCUMENT_JOB_TYPE,
                        claim.claim_token,
                    ],
                )?;
                if changed != 1 {
                    return Err(io::Error::other("parse job is no longer claimed").into());
                }
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn queued_reconcile_document_ids(&mut self) -> StoreResult<Vec<String>> {
        self.recover_expired_review_jobs()?;
        let mut statement = self.connection.prepare(
            "SELECT related_source_document_id FROM jobs \
             WHERE job_type = ?1 AND status = 'queued' \
             ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([RECONCILE_DOCUMENT_JOB_TYPE], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub(crate) fn start_reconcile_document(&mut self, document_id: &str) -> StoreResult<bool> {
        let changed = self.connection.execute(
            "UPDATE jobs \
             SET status = 'running', attempts = attempts + 1, \
                 lease_owner = 'reconcile-document', \
                 lease_until = datetime('now', ?2), \
                 started_at = COALESCE(started_at, CURRENT_TIMESTAMP), updated_at = CURRENT_TIMESTAMP \
             WHERE related_source_document_id = ?1 AND job_type = ?3 \
               AND status = 'queued' AND attempts < max_attempts",
            params![
                document_id,
                format!("+{RECONCILE_DOCUMENT_LEASE_SECONDS} seconds"),
                RECONCILE_DOCUMENT_JOB_TYPE
            ],
        )?;
        if changed == 0 {
            self.connection.execute(
                "UPDATE jobs SET status = 'failed', blocked_reason = 'retry_limit_reached', \
                         finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
                 WHERE related_source_document_id = ?1 AND job_type = ?2 \
                   AND status = 'queued' AND attempts >= max_attempts",
                params![document_id, RECONCILE_DOCUMENT_JOB_TYPE],
            )?;
        }
        Ok(changed == 1)
    }

    pub(crate) fn reconcile_document(&mut self, document_id: &str) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        let review_items_created = transaction.execute(
            "INSERT INTO review_items(id, external_record_id, reason_code, status) \
             SELECT ?1 || ':' || external_records.id, external_records.id, ?2, 'open' \
             FROM external_records \
             WHERE source_document_id = ?3 AND status IN ('staged', 'review') \
               AND NOT EXISTS ( \
                 SELECT 1 FROM review_items \
                 WHERE review_items.external_record_id = external_records.id \
                   AND review_items.reason_code = ?2 AND review_items.status = 'open' \
               )",
            params![
                new_database_id("review"),
                "normalization_profile_unqualified",
                document_id,
            ],
        )?;
        transaction.execute(
            "UPDATE external_records SET status = 'review' \
             WHERE source_document_id = ?1 AND status = 'staged'",
            [document_id],
        )?;
        let changed = transaction.execute(
            "UPDATE jobs SET status = 'succeeded', \
                     result_json = ?1, error_json = NULL, blocked_reason = NULL, \
                     lease_owner = NULL, lease_until = NULL, finished_at = CURRENT_TIMESTAMP, \
                     updated_at = CURRENT_TIMESTAMP \
             WHERE related_source_document_id = ?2 AND job_type = ?3 \
               AND status = 'running' AND lease_owner = 'reconcile-document'",
            params![
                serde_json::json!({
                    "reviewItemsCreated": review_items_created,
                    "policy": "normalization_profile_unqualified"
                })
                .to_string(),
                document_id,
                RECONCILE_DOCUMENT_JOB_TYPE,
            ],
        )?;
        if changed != 1 {
            return Err(io::Error::other("reconcile job is no longer claimed").into());
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn fail_reconcile_document(
        &mut self,
        document_id: &str,
        reason: &'static str,
    ) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs SET status = 'failed', error_json = ?1, blocked_reason = ?2, \
                     lease_owner = NULL, lease_until = NULL, finished_at = CURRENT_TIMESTAMP, \
                     updated_at = CURRENT_TIMESTAMP \
             WHERE related_source_document_id = ?3 AND job_type = ?4 \
               AND status = 'running' AND lease_owner = 'reconcile-document'",
            params![
                serde_json::json!({ "errorCode": reason }).to_string(),
                reason,
                document_id,
                RECONCILE_DOCUMENT_JOB_TYPE,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn block_parse_document_job(
        &mut self,
        claim: &ParseDocumentClaim,
        reason: &'static str,
    ) -> StoreResult<()> {
        let changed = self.connection.execute(
            "UPDATE jobs SET status = 'blocked', blocked_reason = ?1, lease_owner = NULL, \
                     lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?2 AND related_source_document_id = ?3 AND job_type = ?4 \
               AND status = 'running' AND lease_owner = ?5",
            params![
                reason,
                claim.job_id,
                claim.document_id,
                PARSE_DOCUMENT_JOB_TYPE,
                claim.claim_token,
            ],
        )?;
        parse_job_update(changed)
    }

    pub(crate) fn fail_parse_document_job(
        &mut self,
        claim: &ParseDocumentClaim,
        reason: &'static str,
    ) -> StoreResult<()> {
        let changed = self.connection.execute(
            "UPDATE jobs SET \
                 status = CASE WHEN attempts < max_attempts THEN 'queued' ELSE 'failed' END, \
                 result_json = NULL, \
                 error_json = CASE WHEN attempts < max_attempts THEN NULL ELSE ?1 END, \
                 blocked_reason = CASE WHEN attempts < max_attempts THEN NULL ELSE ?2 END, \
                 lease_owner = NULL, lease_until = NULL, \
                 finished_at = CASE WHEN attempts < max_attempts THEN NULL ELSE CURRENT_TIMESTAMP END, \
                 updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?3 AND related_source_document_id = ?4 AND job_type = ?5 \
               AND status = 'running' AND lease_owner = ?6",
            params![
                serde_json::json!({ "errorCode": reason }).to_string(),
                reason,
                claim.job_id,
                claim.document_id,
                PARSE_DOCUMENT_JOB_TYPE,
                claim.claim_token,
            ],
        )?;
        parse_job_update(changed)
    }

    pub fn delete_source_document(&mut self, document_id: &str, audit_id: &str) -> StoreResult<()> {
        if document_id.is_empty() || audit_id.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "source deletion identifiers must not be empty",
            )
            .into());
        }
        let document = find_document_by_id(&self.connection, document_id)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "source document not found"))?;
        if document.file_state != "available" {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "source document file is unavailable",
            )
            .into());
        }
        let locator = document.encrypted_locator.as_deref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "available source document has no encrypted locator",
            )
        })?;
        if !self
            .files
            .verifies(&self.master_key, locator, &document.file_sha256)?
        {
            mark_missing(&mut self.connection, &document, &document.file_sha256)?;
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "source document file is unavailable",
            )
            .into());
        }
        persist_source_deletion(&mut self.connection, &document, audit_id)?;
        self.files.remove(locator)?;
        Ok(())
    }

    pub fn list_documents(&self, money_source_id: &str) -> StoreResult<Vec<SourceDocumentView>> {
        let mut statement = self.connection.prepare(
            "SELECT id, money_source_id, file_sha256, semantic_document_key, original_filename, \
                    mime_type, byte_size, encrypted_locator, file_state, received_at \
             FROM source_documents \
             WHERE money_source_id = ?1 \
             ORDER BY received_at DESC, id",
        )?;
        let rows = statement.query_map([money_source_id], source_document_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn list_unassigned_documents(&self) -> StoreResult<Vec<SourceDocumentView>> {
        let mut statement = self.connection.prepare(
            "SELECT id, money_source_id, file_sha256, semantic_document_key, original_filename, \
                    mime_type, byte_size, encrypted_locator, file_state, received_at \
             FROM source_documents \
             WHERE money_source_id IS NULL \
             ORDER BY received_at DESC, id",
        )?;
        let rows = statement.query_map([], source_document_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn source_document_parse_status(
        &self,
        document_id: &str,
    ) -> StoreResult<Option<(String, Option<String>)>> {
        self.connection
            .query_row(
                "SELECT status, blocked_reason FROM jobs WHERE related_source_document_id = ?1 \
                 AND job_type = ?2 ORDER BY created_at DESC, id DESC LIMIT 1",
                params![document_id, PARSE_DOCUMENT_JOB_TYPE],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn list_review_items(&self) -> StoreResult<Vec<ReviewItemSummary>> {
        let mut statement = self.connection.prepare(
            "SELECT review_items.id, external_records.id, external_records.version, \
                    review_items.reason_code, external_records.amount_value, \
                    external_records.currency, external_records.event_type, \
                    external_records.posted_on, COALESCE(accounts.display_name, 'Unassigned'), \
                    external_records.status = 'committed' \
             FROM review_items \
             JOIN external_records ON external_records.id = review_items.external_record_id \
             LEFT JOIN accounts ON accounts.id = external_records.account_id \
             WHERE review_items.status = 'open' \
               AND external_records.status IN ('staged', 'review', 'committed') \
               AND (accounts.status IS NULL OR accounts.status <> 'dismissed') \
             ORDER BY external_records.posted_on, review_items.id",
        )?;
        let rows = statement.query_map([], review_item_summary_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn review_item_detail(
        &self,
        review_item_id: &str,
    ) -> StoreResult<Option<ReviewItemDetail>> {
        let detail = self
            .connection
            .query_row(
                "SELECT review_items.id, external_records.id, external_records.version, \
                        review_items.reason_code, external_records.amount_value, \
                        external_records.currency, external_records.event_type, \
                        external_records.posted_on, COALESCE(accounts.display_name, 'Unassigned'), \
                        source_documents.original_filename, \
                        COALESCE(money_sources.display_name, 'Unassigned'), \
                        external_records.status = 'committed' \
                 FROM review_items \
                 JOIN external_records ON external_records.id = review_items.external_record_id \
                 JOIN source_documents ON source_documents.id = external_records.source_document_id \
                 LEFT JOIN accounts ON accounts.id = external_records.account_id \
                 LEFT JOIN money_sources ON money_sources.id = source_documents.money_source_id \
                 WHERE review_items.id = ?1 \
                   AND review_items.status = 'open' \
                   AND external_records.status IN ('staged', 'review', 'committed')",
                [review_item_id],
                review_item_detail_from_row,
            )
            .optional()?;
        Ok(detail)
    }

    pub(crate) fn list_recent_activity(&self) -> StoreResult<Vec<RecentActivitySummary>> {
        let mut statement = self.connection.prepare(
            "SELECT id, event_type, event_date, event_class, reverses_event_id, \
                    NOT EXISTS(SELECT 1 FROM ledger_events reversal \
                               WHERE reversal.reverses_event_id = ledger_events.id) \
             FROM ledger_events \
             WHERE status = 'committed' \
             ORDER BY event_date DESC, created_at DESC, id DESC \
             LIMIT 50",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, bool>(5)?,
            ))
        })?;
        let mut activity = Vec::new();
        for row in rows {
            let (event_id, event_type, event_date, event_class, reverses_event_id, has_no_reversal) =
                row?;
            let mut sources = self.connection.prepare(
                "SELECT DISTINCT COALESCE(money_sources.display_name, 'Unassigned') \
                 FROM match_edges \
                 JOIN external_records ON external_records.id = match_edges.external_record_id \
                 JOIN source_documents ON source_documents.id = external_records.source_document_id \
                 LEFT JOIN money_sources ON money_sources.id = source_documents.money_source_id \
                 WHERE match_edges.ledger_event_id = ?1 \
                 ORDER BY 1",
            )?;
            let source_labels = sources
                .query_map([&event_id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            let can_undo = matches!(
                event_type.as_str(),
                "same_currency_transfer" | "credit_card_repayment"
            ) && event_class == "posting"
                && reverses_event_id.is_none()
                && has_no_reversal;
            activity.push(RecentActivitySummary {
                can_undo,
                event_date,
                event_id,
                spending: event_type == "purchase" || event_type == "credit_card_purchase",
                event_type,
                source_labels,
            });
        }
        Ok(activity)
    }

    pub(crate) fn money_overview(&self) -> StoreResult<MoneyOverview> {
        let mut statement = self.connection.prepare(
            "SELECT account_id, account_label, account_type, currency, balance_value, event_date \
             FROM ( \
               SELECT ledger_legs.account_id AS account_id, accounts.display_name AS account_label, \
                      accounts.account_type AS account_type, ledger_legs.currency AS currency, \
                      ledger_legs.balance_value AS balance_value, ledger_events.event_date AS event_date, \
                      ROW_NUMBER() OVER ( \
                        PARTITION BY ledger_legs.account_id, ledger_legs.currency \
                        ORDER BY ledger_events.event_date DESC, ledger_events.created_at DESC, ledger_events.id DESC \
                      ) AS rank \
               FROM ledger_legs \
               JOIN ledger_events ON ledger_events.id = ledger_legs.ledger_event_id \
               JOIN accounts ON accounts.id = ledger_legs.account_id \
               WHERE ledger_events.status = 'committed' \
                 AND ledger_events.event_class = 'observation' \
                 AND ledger_legs.balance_value IS NOT NULL \
             ) WHERE rank = 1 \
             ORDER BY account_label, account_id, currency",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut assets = Vec::new();
        let mut liabilities = Vec::new();
        for row in rows {
            let (account_id, account_label, account_type, currency, value, as_of) = row?;
            let amount = MoneyOverviewAmount {
                account_id,
                account_label,
                as_of,
                currency,
                value,
            };
            if account_type == "credit_card" || account_type == "manual_liability" {
                liabilities.push(amount);
            } else {
                assets.push(amount);
            }
        }
        Ok(MoneyOverview {
            assets,
            liabilities,
        })
    }

    pub(crate) fn list_money_sources(&self) -> StoreResult<Vec<MoneySourceView>> {
        let mut statement = self.connection.prepare(
            "SELECT id, display_name, source_type FROM money_sources \
             ORDER BY display_name, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(MoneySourceView {
                money_source_id: row.get(0)?,
                display_name: row.get(1)?,
                source_type: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn source_document_read_plan(
        &self,
        document_id: &str,
    ) -> StoreResult<imports::SourceDocumentReadPlan> {
        let document = self
            .connection
            .query_row(
                "SELECT file_sha256, mime_type, encrypted_locator, file_state \
                 FROM source_documents WHERE id = ?1",
                [document_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((file_sha256, mime_type, encrypted_locator, file_state)) = document else {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        };
        if file_state != "available" {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "source document file is unavailable",
            )
            .into());
        }
        let encrypted_locator = encrypted_locator.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "available source document has no encrypted locator",
            )
        })?;
        Ok(imports::SourceDocumentReadPlan {
            encrypted_locator,
            file_sha256,
            files: self.files.clone(),
            master_key: self.master_key.clone(),
            mime_type,
        })
    }

    pub fn source_document_mime_type(&self, document_id: &str) -> StoreResult<String> {
        let document = self
            .connection
            .query_row(
                "SELECT mime_type, file_state FROM source_documents WHERE id = ?1",
                [document_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((mime_type, file_state)) = document else {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        };
        if file_state != "available" {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "source document file is unavailable",
            )
            .into());
        }
        Ok(mime_type)
    }

    pub(crate) fn statement_password_state(
        &self,
        money_source_id: &str,
    ) -> StoreResult<Option<StatementPasswordState>> {
        let state = self
            .connection
            .query_row(
                "SELECT money_source_id, secret_storage_key, status \
                 FROM statement_secret_refs WHERE money_source_id = ?1",
                [money_source_id],
                statement_password_state_from_row,
            )
            .optional()?;
        Ok(state)
    }

    pub(crate) fn statement_password_sources(&self) -> StoreResult<Vec<StatementPasswordSource>> {
        let mut statement = self.connection.prepare(
            "SELECT money_sources.id, money_sources.display_name, \
                    EXISTS( \
                      SELECT 1 FROM statement_secret_refs \
                      WHERE statement_secret_refs.money_source_id = money_sources.id \
                        AND statement_secret_refs.status = 'saved' \
                    ) \
             FROM money_sources \
             ORDER BY money_sources.display_name, money_sources.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(StatementPasswordSource {
                money_source_id: row.get(0)?,
                display_name: row.get(1)?,
                has_saved_password: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn pending_statement_password_states(
        &self,
    ) -> StoreResult<Vec<StatementPasswordState>> {
        let mut statement = self.connection.prepare(
            "SELECT money_source_id, secret_storage_key, status \
             FROM statement_secret_refs WHERE status <> 'saved' ORDER BY money_source_id",
        )?;
        let rows = statement.query_map([], statement_password_state_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub(crate) fn begin_statement_password_save(
        &self,
        money_source_id: &str,
        secret_storage_key: &str,
    ) -> StoreResult<()> {
        self.connection.execute(
            "INSERT INTO statement_secret_refs( \
               id, money_source_id, secret_storage_key, status, hint_label \
             ) VALUES (?1, ?2, ?3, 'pending_save', NULL)",
            params![
                format!("statement-secret-ref:{money_source_id}"),
                money_source_id,
                secret_storage_key
            ],
        )?;
        Ok(())
    }

    pub(crate) fn mark_statement_password_saved(&self, money_source_id: &str) -> StoreResult<()> {
        update_statement_password_status(&self.connection, money_source_id, "pending_save", "saved")
    }

    pub(crate) fn begin_statement_password_delete(&self, money_source_id: &str) -> StoreResult<()> {
        update_statement_password_status(
            &self.connection,
            money_source_id,
            "saved",
            "pending_delete",
        )
    }

    pub(crate) fn remove_statement_password_ref(
        &self,
        money_source_id: &str,
        expected_status: &str,
    ) -> StoreResult<()> {
        let changed = self.connection.execute(
            "DELETE FROM statement_secret_refs WHERE money_source_id = ?1 AND status = ?2",
            params![money_source_id, expected_status],
        )?;
        if changed != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "statement password state changed",
            )
            .into());
        }
        Ok(())
    }

    pub(crate) fn apply_trusted_classification_for_parse_job(
        &mut self,
        input: &TrustedDocumentClassification<'_>,
        claim: &ParseDocumentClaim,
    ) -> StoreResult<SourceDocumentRoutingOutcome> {
        self.apply_trusted_classification_with_parse_job(input, Some(claim))
    }

    fn apply_trusted_classification_with_parse_job(
        &mut self,
        input: &TrustedDocumentClassification<'_>,
        parse_job: Option<&ParseDocumentClaim>,
    ) -> StoreResult<SourceDocumentRoutingOutcome> {
        validate_classification(input)?;
        let transaction = self.connection.transaction()?;
        if let Some(claim) = parse_job
            && !parse_job_claimed(&transaction, claim)?
        {
            return Err(io::Error::other("parse job is no longer claimed").into());
        }
        let existing_identity = transaction
            .query_row(
                "SELECT money_source_id, semantic_document_key \
                 FROM source_documents WHERE id = ?1",
                [input.document_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()?;
        let Some((assigned_source_id, semantic_document_key)) = existing_identity else {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        };

        let source_ids = if let Some(provider_root_id) = input.provider_root_id {
            let mut source_statement = transaction.prepare(
                "SELECT id FROM money_sources \
                 WHERE provider_key = ?1 AND provider_root_id = ?2 \
                 ORDER BY id LIMIT 2",
            )?;
            source_statement
                .query_map(params![input.provider_key, provider_root_id], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut source_statement = transaction.prepare(
                "SELECT id FROM money_sources \
                 WHERE provider_key = ?1 AND provider_root_id IS NULL \
                 ORDER BY id LIMIT 2",
            )?;
            source_statement
                .query_map([input.provider_key], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        let money_source_id = match source_ids.as_slice() {
            [] => {
                drop(transaction);
                let candidate_id = new_database_id("source-candidate");
                let scope = match input.provider_root_id {
                    Some(root_id) => {
                        crate::database::intake::MoneySourceCandidateScope::ProviderRootId(root_id)
                    }
                    None => crate::database::intake::MoneySourceCandidateScope::ProviderSingleton,
                };
                self.attach_money_source_candidate(
                    &crate::database::intake::MoneySourceCandidateInput {
                        candidate_id: &candidate_id,
                        document_id: input.document_id,
                        provider_key: input.provider_key,
                        scope,
                    },
                )?;
                return Ok(needs_attention(
                    input.document_id,
                    "source_confirmation_required",
                ));
            }
            [money_source_id] => money_source_id,
            _ => return Ok(needs_attention(input.document_id, "money_source_ambiguous")),
        };
        if assigned_source_id
            .as_deref()
            .is_some_and(|id| id != money_source_id)
            || semantic_document_key
                .as_deref()
                .is_some_and(|key| key != input.semantic_document_key)
        {
            return Ok(needs_attention(
                input.document_id,
                "classification_conflict",
            ));
        }

        ensure_fiat_currency_instruments(&transaction, input.accounts)?;

        let mut account_ids = Vec::with_capacity(input.accounts.len());
        for account in input.accounts {
            let Some(provider_account_id) = account.provider_account_id else {
                return Ok(needs_attention(input.document_id, "account_mapping_needed"));
            };
            let existing = transaction
                .query_row(
                    "SELECT id, status FROM accounts \
                     WHERE money_source_id = ?1 AND provider_key = ?2 \
                       AND provider_account_id = ?3 AND status <> 'merged'",
                    params![money_source_id, input.provider_key, provider_account_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?;
            match existing {
                Some((_, status)) if status == "archived" => {
                    return Ok(needs_attention(
                        input.document_id,
                        "account_restore_required",
                    ));
                }
                Some((account_id, _)) => account_ids.push(account_id),
                None => {
                    transaction.execute(
                        "INSERT INTO accounts( \
                           id, money_source_id, provider_key, provider_account_id, account_type, \
                           display_name, masked_identifier, currency, status \
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'candidate')",
                        params![
                            account.account_id,
                            money_source_id,
                            input.provider_key,
                            provider_account_id,
                            account.account_type,
                            account.display_name,
                            account.masked_identifier,
                            account.currency,
                        ],
                    )?;
                    account_ids.push(account.account_id.to_owned());
                }
            }
        }

        transaction.execute(
            "UPDATE source_documents \
             SET money_source_id = ?1, semantic_document_key = ?2, \
                 document_type = ?3, statement_period_from = ?4, statement_period_to = ?5 \
             WHERE id = ?6",
            params![
                money_source_id,
                input.semantic_document_key,
                input.document_type,
                input.statement_period_from,
                input.statement_period_to,
                input.document_id,
            ],
        )?;
        transaction.execute(
            "DELETE FROM source_document_accounts WHERE source_document_id = ?1",
            [input.document_id],
        )?;
        for account_id in &account_ids {
            transaction.execute(
                "INSERT INTO source_document_accounts(source_document_id, account_id) \
                 VALUES (?1, ?2)",
                params![input.document_id, account_id],
            )?;
        }
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES (?1, 'source_document', ?2, 'trusted_classification_applied', \
                       'system', 'provider_and_account_verified', 'classification-v1')",
            params![input.audit_id, input.document_id],
        )?;
        transaction.commit()?;
        Ok(SourceDocumentRoutingOutcome {
            account_ids,
            document_id: input.document_id.to_owned(),
            money_source_id: Some(money_source_id.to_owned()),
            reason: None,
            status: SourceDocumentRoutingStatus::Routed,
        })
    }

    pub(crate) fn persist_validated_structured_parse_for_claimed_job(
        &mut self,
        document_id: &str,
        input: &ValidatedStructuredParseInput,
        claim: &ParseDocumentClaim,
        input_hash: &str,
        output_hash: &str,
    ) -> StoreResult<()> {
        self.persist_validated_structured_parse_with_parse_job(
            document_id,
            input,
            &claim.logical_run_key,
            input_hash,
            output_hash,
            claim,
        )
    }

    fn persist_validated_structured_parse_with_parse_job(
        &mut self,
        document_id: &str,
        input: &ValidatedStructuredParseInput,
        logical_run_key: &str,
        input_hash: &str,
        output_hash: &str,
        parse_job: &ParseDocumentClaim,
    ) -> StoreResult<()> {
        validate_structured_parse_input(document_id, input)?;
        if logical_run_key.is_empty()
            || input_hash.is_empty()
            || output_hash.is_empty()
            || logical_run_key.len() > 256
            || input_hash.len() > 128
            || output_hash.len() > 128
        {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "invalid parse run key").into(),
            );
        }
        let transaction = self.connection.transaction()?;
        if !parse_job_claimed(&transaction, parse_job)? {
            return Err(io::Error::other("parse job is no longer claimed").into());
        }
        let document_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM source_documents WHERE id = ?1)",
            [document_id],
            |row| row.get(0),
        )?;
        if !document_exists {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        }
        let existing_run = transaction
            .query_row(
                "SELECT profile_json, input_hash, output_hash FROM parse_runs \
                 WHERE source_document_id = ?1 AND logical_run_key = ?2",
                params![document_id, logical_run_key],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;
        if let Some((profile_json, existing_input_hash, existing_output_hash)) = existing_run {
            if profile_json != input.profile_json
                || existing_input_hash != input_hash
                || existing_output_hash.as_deref() != Some(output_hash)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "logical parse run changed",
                )
                .into());
            }
            return Ok(());
        }

        let parse_run_id = new_database_id("parse");
        transaction.execute(
            "INSERT INTO parse_runs( \
               id, source_document_id, normalization_profile_id, logical_run_key, profile_json, \
               input_hash, output_hash, status \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'validated')",
            params![
                parse_run_id,
                document_id,
                input.normalization_profile_id,
                logical_run_key,
                input.profile_json,
                input_hash,
                output_hash,
            ],
        )?;
        for record in &input.records {
            let committed_record = transaction
                .query_row(
                    "SELECT id, record_type, event_type, posted_on, amount_value, currency, account_balance_delta, raw_json \
                     FROM external_records \
                     WHERE stable_record_key = ?1 AND status = 'committed' \
                     ORDER BY version DESC LIMIT 1",
                    [&record.stable_record_key],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, String>(7)?,
                        ))
                    },
                )
                .optional()?;

            if let Some((
                committed_id,
                committed_record_type,
                committed_event_type,
                committed_posted_on,
                committed_amount_value,
                committed_currency,
                committed_account_balance_delta,
                committed_raw_json,
            )) = committed_record
            {
                let diverges = committed_record_type != record.record_type
                    || committed_event_type != record.event_type
                    || committed_posted_on != record.posted_on
                    || committed_amount_value != record.amount_value
                    || committed_currency != record.currency
                    || committed_account_balance_delta != record.account_balance_delta;

                if diverges {
                    let review_item_id = new_database_id("review");
                    transaction.execute(
                        "INSERT INTO review_items(id, external_record_id, reason_code, status) \
                         SELECT ?1, ?2, 'reparse_divergence', 'open' \
                         WHERE NOT EXISTS ( \
                           SELECT 1 FROM review_items \
                           WHERE external_record_id = ?2 \
                             AND reason_code = 'reparse_divergence' \
                             AND status = 'open' \
                         )",
                        params![review_item_id, committed_id],
                    )?;
                }

                let old_raw_sha256 = format!("{:x}", Sha256::digest(committed_raw_json.as_bytes()));
                let new_raw_sha256 = format!("{:x}", Sha256::digest(record.raw_json.as_bytes()));
                let source_ref = format!(
                    "{}:{old_raw_sha256}:{new_raw_sha256}",
                    record.stable_record_key
                );

                transaction.execute(
                    "INSERT INTO audit_log( \
                       id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
                     ) VALUES (?1, 'external_record', ?2, 'reparse_skipped_committed_record', 'system', \
                               'reparse', ?3, ?4)",
                    params![
                        new_audit_id(),
                        committed_id,
                        source_ref,
                        REVIEW_POLICY_VERSION,
                    ],
                )?;

                continue;
            }

            let previous = transaction
                .query_row(
                    "SELECT id, version FROM external_records \
                     WHERE stable_record_key = ?1 ORDER BY version DESC LIMIT 1",
                    [&record.stable_record_key],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?;
            let version = previous.as_ref().map_or(1, |(_, version)| version + 1);
            if previous.is_some() {
                transaction.execute(
                    "UPDATE external_records SET status = 'superseded' \
                     WHERE stable_record_key = ?1 AND status IN ('staged', 'review', 'removed')",
                    [&record.stable_record_key],
                )?;
                transaction.execute(
                    "UPDATE review_items SET status = 'resolved' \
                     WHERE external_record_id IN ( \
                       SELECT id FROM external_records \
                       WHERE stable_record_key = ?1 AND status = 'superseded' \
                     ) AND status = 'open'",
                    [&record.stable_record_key],
                )?;
            }
            let dismissed: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = ?1 AND status = 'dismissed')",
                [&record.account_id],
                |row| row.get(0),
            )?;
            transaction.execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                   status, record_type, event_type, posted_on, amount_value, currency, \
                   account_balance_delta, posting_status, raw_json, validation_json \
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    new_database_id("record"),
                    parse_run_id,
                    document_id,
                    record.account_id,
                    record.stable_record_key,
                    version,
                    if dismissed { "removed" } else { "staged" },
                    record.record_type,
                    record.event_type,
                    record.posted_on,
                    record.amount_value,
                    record.currency,
                    record.account_balance_delta,
                    record.posting_status,
                    record.raw_json,
                    record.validation_json,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn edit_review_record(
        &mut self,
        review_item_id: &str,
        expected_record_version: i64,
        posted_on: Option<&str>,
        amount_value: Option<&str>,
        account_balance_delta: Option<&str>,
    ) -> StoreResult<ReviewMutationOutcome> {
        if expected_record_version <= 0
            || !valid_optional_date(posted_on)
            || !valid_optional_non_negative_decimal(amount_value)
            || !valid_optional_decimal(account_balance_delta)
            || (posted_on.is_none() && amount_value.is_none() && account_balance_delta.is_none())
        {
            return Ok(review_conflict("invalid_review_edit"));
        }
        let transaction = self.connection.transaction()?;
        let current = transaction
            .query_row(
                "SELECT external_records.id, external_records.version \
                 FROM review_items \
                 JOIN external_records ON external_records.id = review_items.external_record_id \
                 WHERE review_items.id = ?1 AND review_items.status = 'open' \
                   AND external_records.version = ?2 \
                   AND external_records.status IN ('staged', 'review')",
                params![review_item_id, expected_record_version],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        let Some((record_id, version)) = current else {
            return Ok(review_conflict("stale_review_item"));
        };
        let next_record_id = new_database_id("record");
        let next_review_item_id = new_database_id("review");
        transaction.execute(
            "INSERT INTO external_records( \
               id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
               status, record_type, event_type, posted_on, amount_value, currency, \
               account_balance_delta, raw_json, validation_json \
             ) \
             SELECT ?1, parse_run_id, source_document_id, account_id, stable_record_key, version + 1, \
                    'review', record_type, event_type, COALESCE(?2, posted_on), \
                    COALESCE(?3, amount_value), currency, COALESCE(?4, account_balance_delta), \
                    raw_json, validation_json \
             FROM external_records WHERE id = ?5",
            params![
                next_record_id,
                posted_on,
                amount_value,
                account_balance_delta,
                record_id
            ],
        )?;
        transaction.execute(
            "UPDATE external_records SET status = 'superseded' WHERE id = ?1",
            [&record_id],
        )?;
        transaction.execute(
            "UPDATE review_items SET status = 'resolved' WHERE id = ?1",
            [review_item_id],
        )?;
        transaction.execute(
            "UPDATE review_relationships SET status = 'invalidated' \
             WHERE status = 'accepted' \
               AND (first_external_record_id = ?1 OR second_external_record_id = ?1)",
            [&record_id],
        )?;
        transaction.execute(
            "INSERT INTO review_items(id, external_record_id, reason_code, status) \
             VALUES (?1, ?2, 'record_edited', 'open')",
            params![next_review_item_id, next_record_id],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) VALUES (?1, 'external_record', ?2, 'review_record_edited', 'user', \
                       'review_edit', ?3, ?4)",
            params![
                new_audit_id(),
                next_record_id,
                format!("{record_id}:{version}"),
                REVIEW_POLICY_VERSION,
            ],
        )?;
        transaction.commit()?;
        Ok(ReviewMutationOutcome {
            reason: None,
            record_version: Some(version + 1),
            review_item_id: Some(next_review_item_id),
            status: ReviewMutationStatus::Updated,
        })
    }

    pub(crate) fn remove_review_record(
        &mut self,
        review_item_id: &str,
        expected_record_version: i64,
    ) -> StoreResult<ReviewMutationOutcome> {
        if expected_record_version <= 0 {
            return Ok(review_conflict("invalid_review_request"));
        }
        let transaction = self.connection.transaction()?;
        let current = transaction
            .query_row(
                "SELECT external_records.id FROM review_items \
                 JOIN external_records ON external_records.id = review_items.external_record_id \
                 WHERE review_items.id = ?1 AND review_items.status = 'open' \
                   AND external_records.version = ?2 \
                   AND external_records.status IN ('staged', 'review')",
                params![review_item_id, expected_record_version],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(record_id) = current else {
            return Ok(review_conflict("stale_review_item"));
        };
        transaction.execute(
            "UPDATE external_records SET status = 'removed' WHERE id = ?1",
            [&record_id],
        )?;
        transaction.execute(
            "UPDATE review_items SET status = 'dismissed' WHERE id = ?1",
            [review_item_id],
        )?;
        transaction.execute(
            "UPDATE review_relationships SET status = 'invalidated' \
             WHERE status = 'accepted' \
               AND (first_external_record_id = ?1 OR second_external_record_id = ?1)",
            [&record_id],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) VALUES (?1, 'external_record', ?2, 'review_record_removed', 'user', \
                       'review_remove', ?3, ?4)",
            params![
                new_audit_id(),
                record_id,
                format!("{review_item_id}:{expected_record_version}"),
                REVIEW_POLICY_VERSION,
            ],
        )?;
        transaction.commit()?;
        Ok(ReviewMutationOutcome {
            reason: None,
            record_version: None,
            review_item_id: None,
            status: ReviewMutationStatus::Removed,
        })
    }

    pub(crate) fn relationship_candidate_input(
        &self,
        review_item_id: &str,
        expected_record_version: i64,
    ) -> StoreResult<Option<ReviewRelationshipCandidateInput>> {
        let Some((record_id, event_type, record)) =
            self.open_review_core_record(review_item_id, expected_record_version)?
        else {
            return Ok(None);
        };
        let Some(event_type) = event_type.filter(|value| is_review_event_type(value)) else {
            return Ok(Some(ReviewRelationshipCandidateInput {
                candidates: Vec::new(),
                event_type: String::new(),
                record,
            }));
        };
        let mut statement = self.connection.prepare(
            "WITH candidate_records AS ( \
               SELECT external_records.id, external_records.account_id, accounts.account_type, \
                      external_records.currency, external_records.posted_on, \
                      external_records.account_balance_delta, instruments.id AS instrument_id, \
                      CASE \
                        WHEN instr(ltrim(external_records.account_balance_delta, '-'), '.') = 0 \
                          THEN ltrim(external_records.account_balance_delta, '-') \
                        ELSE rtrim(rtrim(ltrim(external_records.account_balance_delta, '-'), '0'), '.') \
                      END AS delta_magnitude \
               FROM external_records \
               JOIN accounts ON accounts.id = external_records.account_id \
               JOIN instruments ON instruments.currency = external_records.currency \
                  AND instruments.instrument_type = 'fiat_currency' \
               WHERE external_records.id <> ?1 \
                 AND external_records.status IN ('staged', 'review') \
                 AND external_records.event_type = ?2 \
                 AND external_records.currency = ?3 \
                 AND julianday(external_records.posted_on) BETWEEN julianday(?4) - ?5 \
                     AND julianday(?4) + ?5 \
             ) \
             SELECT id, account_id, account_type, currency, posted_on, \
                    account_balance_delta, instrument_id \
             FROM candidate_records \
             WHERE account_id <> ?6 \
               AND delta_magnitude <> '0' \
               AND delta_magnitude = CASE \
                 WHEN instr(ltrim(?8, '-'), '.') = 0 THEN ltrim(?8, '-') \
                 ELSE rtrim(rtrim(ltrim(?8, '-'), '0'), '.') \
               END \
               AND ( \
                 (?2 = 'same_currency_transfer' AND ( \
                   (?8 GLOB '-*' AND account_balance_delta NOT GLOB '-*') \
                   OR (?8 NOT GLOB '-*' AND account_balance_delta GLOB '-*') \
                 )) \
                 OR (?2 = 'credit_card_repayment' \
                   AND ?8 GLOB '-*' AND account_balance_delta GLOB '-*' \
                   AND ( \
                     (?7 = 'credit_card' \
                       AND account_type IN ('deposit_account', 'currency_balance', 'cash_balance')) \
                     OR (?7 IN ('deposit_account', 'currency_balance', 'cash_balance') \
                       AND account_type = 'credit_card') \
                   )) \
               ) \
             ORDER BY ABS(julianday(posted_on) - julianday(?4)), posted_on, id \
             LIMIT 100",
        )?;
        let candidates = statement
            .query_map(
                params![
                    record_id,
                    event_type,
                    record.currency,
                    record.posted_on,
                    MAX_SUPPORTED_RELATIONSHIP_WINDOW_DAYS,
                    record.account_id,
                    record.account_type,
                    record.account_balance_delta,
                ],
                core_review_record_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(ReviewRelationshipCandidateInput {
            candidates,
            event_type,
            record,
        }))
    }

    pub(crate) fn relationship_candidate_summaries(
        &self,
        candidate_ids: &[String],
    ) -> StoreResult<Vec<RelationshipCandidateSummary>> {
        let mut candidates = Vec::new();
        for candidate_id in candidate_ids {
            let candidate = self
                .connection
                .query_row(
                    "SELECT external_records.id, external_records.version, external_records.event_type, \
                            external_records.posted_on, external_records.amount_value, \
                            external_records.account_balance_delta, external_records.currency, \
                            accounts.display_name \
                     FROM external_records \
                     JOIN accounts ON accounts.id = external_records.account_id \
                     WHERE external_records.id = ?1 \
                       AND external_records.status IN ('staged', 'review')",
                    [candidate_id],
                    relationship_candidate_from_row,
                )
                .optional()?;
            if let Some(candidate) = candidate {
                candidates.push(candidate);
            }
        }
        Ok(candidates)
    }

    pub(crate) fn accept_review_relationship(
        &mut self,
        review_item_id: &str,
        expected_record_version: i64,
        candidate_record_id: &str,
        expected_candidate_version: i64,
        event: &CorePreparedReviewEvent,
    ) -> StoreResult<ReviewMutationOutcome> {
        if expected_record_version <= 0
            || expected_candidate_version <= 0
            || !valid_core_review_event(event)
        {
            return Ok(review_conflict("invalid_relationship_request"));
        }
        let transaction = self.connection.transaction()?;
        let primary = open_review_record_for_relationship(
            &transaction,
            review_item_id,
            expected_record_version,
        )?;
        let Some(primary) = primary else {
            return Ok(review_conflict("stale_review_item"));
        };
        let candidate = open_review_record_by_id(
            &transaction,
            candidate_record_id,
            expected_candidate_version,
        )?;
        let Some(candidate) = candidate else {
            return Ok(review_conflict("stale_relationship_candidate"));
        };
        if primary.record_id == candidate.record_id
            || primary.event_type.as_deref() != Some(event.event_type.as_str())
            || candidate.event_type.as_deref() != Some(event.event_type.as_str())
            || !same_record_ids(
                &event.source_record_ids,
                &primary.record_id,
                &candidate.record_id,
            )
        {
            return Ok(review_conflict("relationship_changed"));
        }
        let Some(ref candidate_review_item_id) = candidate.review_item_id else {
            return Ok(review_conflict("relationship_needs_review"));
        };
        let (first, second) = ordered_relationship_records(&primary, &candidate);
        let existing = transaction
            .query_row(
                "SELECT id FROM review_relationships \
                 WHERE first_external_record_id = ?1 AND second_external_record_id = ?2",
                params![first.record_id, second.record_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if existing.is_some() {
            return Ok(ReviewMutationOutcome {
                reason: None,
                record_version: None,
                review_item_id: None,
                status: ReviewMutationStatus::RelationshipAccepted,
            });
        }
        let allocation_value = decimal_magnitude(&primary.account_balance_delta)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid core amount"))?;
        transaction.execute(
            "INSERT INTO review_relationships( \
               id, event_type, first_external_record_id, second_external_record_id, \
               first_review_item_id, second_review_item_id, allocation_value, unit, status \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'accepted')",
            params![
                new_database_id("relationship"),
                event.event_type,
                first.record_id,
                second.record_id,
                first.review_item_id.as_deref().unwrap_or(review_item_id),
                second
                    .review_item_id
                    .as_deref()
                    .unwrap_or(candidate_review_item_id),
                allocation_value,
                primary.currency,
            ],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) VALUES (?1, 'review_relationship', ?2, 'review_relationship_accepted', \
                       'user', 'confirmed_relationship', ?3, ?4)",
            params![
                new_audit_id(),
                format!("{}:{}", first.record_id, second.record_id),
                event.event_type,
                REVIEW_POLICY_VERSION,
            ],
        )?;
        transaction.commit()?;
        Ok(ReviewMutationOutcome {
            reason: None,
            record_version: None,
            review_item_id: None,
            status: ReviewMutationStatus::RelationshipAccepted,
        })
    }

    pub(crate) fn enqueue_commit_review_batch(
        &mut self,
        review_item_ids: &[String],
    ) -> StoreResult<ReviewJobSummary> {
        if review_item_ids.is_empty()
            || review_item_ids.len() > 100
            || review_item_ids.iter().any(|value| value.is_empty())
            || review_item_ids.iter().collect::<HashSet<_>>().len() != review_item_ids.len()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "review batch must select between one and one hundred unique review items",
            )
            .into());
        }
        let job_id = new_database_id("job");
        let input_json = serde_json::to_string(&CommitReviewBatchInput {
            review_item_ids: review_item_ids.to_vec(),
        })?;
        self.connection.execute(
            "INSERT INTO jobs(id, job_type, status, input_json, related_review_item_id) \
             VALUES (?1, ?2, 'queued', ?3, ?4)",
            params![
                job_id,
                COMMIT_REVIEW_BATCH_JOB_TYPE,
                input_json,
                review_item_ids[0]
            ],
        )?;
        self.review_job(&job_id)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "new review job was not persisted",
            )
            .into()
        })
    }

    pub(crate) fn review_job(&self, job_id: &str) -> StoreResult<Option<ReviewJobSummary>> {
        let row = self
            .connection
            .query_row(
                "SELECT id, status, created_at, finished_at, result_json \
                 FROM jobs WHERE id = ?1 AND job_type = ?2",
                params![job_id, COMMIT_REVIEW_BATCH_JOB_TYPE],
                review_job_summary_from_row,
            )
            .optional()?;
        Ok(row)
    }

    pub(crate) fn queued_review_job_ids(&self) -> StoreResult<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT id FROM jobs \
             WHERE job_type = ?1 AND status = 'queued' \
             ORDER BY created_at, id",
        )?;
        statement
            .query_map([COMMIT_REVIEW_BATCH_JOB_TYPE], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub(crate) fn recover_expired_review_jobs(&mut self) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs \
             SET status = 'queued', lease_owner = NULL, lease_until = NULL, updated_at = CURRENT_TIMESTAMP \
             WHERE job_type IN (?1, ?2) AND status = 'running' \
               AND lease_until IS NOT NULL AND lease_until <= CURRENT_TIMESTAMP",
            params![COMMIT_REVIEW_BATCH_JOB_TYPE, RECONCILE_DOCUMENT_JOB_TYPE],
        )?;
        Ok(())
    }

    pub(crate) fn recover_interrupted_review_jobs(&mut self) -> StoreResult<()> {
        // The app holds exclusive process ownership of the Vault, so these
        // running claims cannot still belong to a live peer at open.
        self.connection.execute(
            "UPDATE jobs \
             SET status = 'queued', lease_owner = NULL, lease_until = NULL, updated_at = CURRENT_TIMESTAMP \
             WHERE job_type IN (?1, ?2) AND status = 'running'",
            params![COMMIT_REVIEW_BATCH_JOB_TYPE, RECONCILE_DOCUMENT_JOB_TYPE],
        )?;
        Ok(())
    }

    pub(crate) fn recover_expired_parse_document_jobs(&mut self) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs \
             SET status = 'queued', lease_owner = NULL, lease_until = NULL, updated_at = CURRENT_TIMESTAMP \
             WHERE job_type = ?1 AND status = 'running' \
               AND lease_until IS NOT NULL AND lease_until <= CURRENT_TIMESTAMP",
            [PARSE_DOCUMENT_JOB_TYPE],
        )?;
        Ok(())
    }

    pub(crate) fn claim_review_batch(
        &mut self,
        job_id: &str,
        lease_owner: &str,
    ) -> StoreResult<Option<ClaimedReviewBatch>> {
        if lease_owner.is_empty() {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "lease owner is required").into(),
            );
        }
        let transaction = self.connection.transaction()?;
        let row = transaction
            .query_row(
                "SELECT status, attempts, max_attempts, input_json \
                 FROM jobs WHERE id = ?1 AND job_type = ?2",
                params![job_id, COMMIT_REVIEW_BATCH_JOB_TYPE],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((status, attempts, max_attempts, input_json)) = row else {
            return Ok(None);
        };
        if status != "queued" {
            return Ok(None);
        }
        if attempts >= max_attempts {
            transaction.execute(
                "UPDATE jobs SET status = 'failed', finished_at = CURRENT_TIMESTAMP, \
                         blocked_reason = 'retry_limit_reached', updated_at = CURRENT_TIMESTAMP \
                 WHERE id = ?1",
                [job_id],
            )?;
            transaction.commit()?;
            return Ok(None);
        }
        let input: CommitReviewBatchInput = serde_json::from_str(&input_json)?;
        transaction.execute(
            "UPDATE jobs SET status = 'running', attempts = attempts + 1, \
                     lease_owner = ?1, lease_until = datetime('now', ?2), \
                     started_at = COALESCE(started_at, CURRENT_TIMESTAMP), updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?3 AND status = 'queued'",
            params![
                lease_owner,
                format!("+{COMMIT_REVIEW_BATCH_LEASE_SECONDS} seconds"),
                job_id
            ],
        )?;
        transaction.commit()?;
        Ok(Some(ClaimedReviewBatch {
            job_id: job_id.to_owned(),
            lease_owner: lease_owner.to_owned(),
            review_item_ids: input.review_item_ids,
        }))
    }

    pub(crate) fn prepare_commit_review_groups(
        &self,
        claimed: &ClaimedReviewBatch,
    ) -> StoreResult<(Vec<CommitReviewGroup>, Vec<ReviewBatchGroupOutcome>)> {
        let mut groups = Vec::new();
        let mut outcomes = Vec::new();
        let mut seen_relationships = HashSet::new();
        let selected_review_items = claimed
            .review_item_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        for review_item_id in &claimed.review_item_ids {
            let record_id = self.review_item_record_id(review_item_id)?;
            let Some(record_id) = record_id else {
                outcomes.push(ReviewBatchGroupOutcome {
                    reason: Some("stale_review_item".to_owned()),
                    record_ids: Vec::new(),
                    status: ReviewBatchGroupStatus::Stale,
                });
                continue;
            };
            let relationships = self.relationships_for_review_item(review_item_id)?;
            let accepted = relationships
                .iter()
                .filter(|relationship| relationship.status == "accepted")
                .collect::<Vec<_>>();
            let committed = relationships
                .iter()
                .filter(|relationship| relationship.status == "committed")
                .collect::<Vec<_>>();
            if accepted.len() > 1 {
                outcomes.push(ReviewBatchGroupOutcome {
                    reason: Some("ambiguous_relationship".to_owned()),
                    record_ids: vec![record_id],
                    status: ReviewBatchGroupStatus::StillNeedsReview,
                });
                continue;
            }
            if let Some(relationship) = committed.first() {
                if seen_relationships.insert(relationship.id.clone()) {
                    outcomes.push(ReviewBatchGroupOutcome {
                        reason: None,
                        record_ids: relationship.record_ids(),
                        status: ReviewBatchGroupStatus::AlreadyCommitted,
                    });
                }
                continue;
            }
            let Some(relationship) = accepted.first() else {
                outcomes.push(ReviewBatchGroupOutcome {
                    reason: Some("relationship_not_confirmed".to_owned()),
                    record_ids: vec![record_id],
                    status: ReviewBatchGroupStatus::StillNeedsReview,
                });
                continue;
            };
            if !selected_review_items.contains(relationship.first_review_item_id.as_str())
                || !selected_review_items.contains(relationship.second_review_item_id.as_str())
            {
                if seen_relationships.insert(relationship.id.clone()) {
                    outcomes.push(ReviewBatchGroupOutcome {
                        reason: Some("relationship_not_selected".to_owned()),
                        record_ids: vec![record_id],
                        status: ReviewBatchGroupStatus::StillNeedsReview,
                    });
                }
                continue;
            }
            if !seen_relationships.insert(relationship.id.clone()) {
                continue;
            }
            let first = self.core_record_by_id(&relationship.first_record_id)?;
            let second = self.core_record_by_id(&relationship.second_record_id)?;
            let (Some(first), Some(second)) = (first, second) else {
                outcomes.push(ReviewBatchGroupOutcome {
                    reason: Some("stale_relationship".to_owned()),
                    record_ids: relationship.record_ids(),
                    status: ReviewBatchGroupStatus::Stale,
                });
                continue;
            };
            groups.push(CommitReviewGroup {
                event_type: relationship.event_type.clone(),
                records: [first, second],
                relationship_id: relationship.id.clone(),
                review_item_ids: [
                    relationship.first_review_item_id.clone(),
                    relationship.second_review_item_id.clone(),
                ],
            });
        }
        Ok((groups, outcomes))
    }

    pub(crate) fn commit_prepared_review_group(
        &mut self,
        claimed: &ClaimedReviewBatch,
        group: &CommitReviewGroup,
        event: &CorePreparedReviewEvent,
    ) -> StoreResult<ReviewBatchGroupOutcome> {
        let fallback_record_ids = group
            .records
            .iter()
            .map(|record| record.id.clone())
            .collect();
        if !valid_core_review_event(event)
            || event.event_type != group.event_type
            || !event_matches_group(event, group)
            || group
                .review_item_ids
                .iter()
                .any(|review_item_id| !claimed.review_item_ids.contains(review_item_id))
        {
            return Ok(ReviewBatchGroupOutcome {
                reason: Some("core_preflight_failed".to_owned()),
                record_ids: fallback_record_ids,
                status: ReviewBatchGroupStatus::StillNeedsReview,
            });
        }
        let transaction = self.connection.transaction()?;
        let lease_is_current = transaction
            .query_row(
                "SELECT 1 FROM jobs WHERE id = ?1 AND job_type = ?2 \
                 AND status = 'running' AND lease_owner = ?3",
                params![
                    claimed.job_id,
                    COMMIT_REVIEW_BATCH_JOB_TYPE,
                    claimed.lease_owner
                ],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !lease_is_current {
            return Ok(ReviewBatchGroupOutcome {
                reason: Some("job_lease_changed".to_owned()),
                record_ids: fallback_record_ids,
                status: ReviewBatchGroupStatus::Stale,
            });
        }
        let relationship = transaction
            .query_row(
                "SELECT id, event_type, first_external_record_id, second_external_record_id, \
                        first_review_item_id, second_review_item_id, allocation_value, unit, status \
                 FROM review_relationships WHERE id = ?1",
                [&group.relationship_id],
                |row| {
                    Ok(StoredReviewRelationship {
                        id: row.get(0)?,
                        event_type: row.get(1)?,
                        first_record_id: row.get(2)?,
                        second_record_id: row.get(3)?,
                        first_review_item_id: row.get(4)?,
                        second_review_item_id: row.get(5)?,
                        allocation_value: row.get(6)?,
                        unit: row.get(7)?,
                        status: row.get(8)?,
                    })
                },
            )
            .optional()?;
        let Some(relationship) = relationship else {
            return Ok(ReviewBatchGroupOutcome {
                reason: Some("stale_relationship".to_owned()),
                record_ids: fallback_record_ids,
                status: ReviewBatchGroupStatus::Stale,
            });
        };
        if relationship.status == "committed" {
            return Ok(ReviewBatchGroupOutcome {
                reason: None,
                record_ids: relationship.record_ids(),
                status: ReviewBatchGroupStatus::AlreadyCommitted,
            });
        }
        if relationship.status != "accepted"
            || relationship.event_type != event.event_type
            || !same_record_ids(
                &event.source_record_ids,
                &relationship.first_record_id,
                &relationship.second_record_id,
            )
        {
            return Ok(ReviewBatchGroupOutcome {
                reason: Some("relationship_changed".to_owned()),
                record_ids: relationship.record_ids(),
                status: ReviewBatchGroupStatus::Stale,
            });
        }
        let current_records = transaction
            .prepare(
                "SELECT id, stable_record_key, version, status FROM external_records \
                 WHERE id IN (?1, ?2) ORDER BY id",
            )?
            .query_map(
                params![relationship.first_record_id, relationship.second_record_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        if current_records.len() != 2
            || current_records
                .iter()
                .any(|(_, _, _, status)| status != "staged" && status != "review")
        {
            return Ok(ReviewBatchGroupOutcome {
                reason: Some("stale_record".to_owned()),
                record_ids: relationship.record_ids(),
                status: ReviewBatchGroupStatus::Stale,
            });
        }
        let committed_sibling_exists: bool = transaction.query_row(
            "SELECT EXISTS( \
               SELECT 1 FROM external_records committed \
               JOIN external_records current \
                 ON current.stable_record_key = committed.stable_record_key \
               WHERE current.id IN (?1, ?2) \
                 AND committed.status = 'committed' \
             )",
            params![relationship.first_record_id, relationship.second_record_id],
            |row| row.get(0),
        )?;
        if committed_sibling_exists {
            return Ok(ReviewBatchGroupOutcome {
                reason: Some("committed_sibling_exists".to_owned()),
                record_ids: relationship.record_ids(),
                status: ReviewBatchGroupStatus::Stale,
            });
        }
        let commit_key = review_commit_key(&event.event_type, &current_records);
        let existing = transaction
            .query_row(
                "SELECT id FROM ledger_events WHERE commit_idempotency_key = ?1",
                [&commit_key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if existing.is_some() {
            return Ok(ReviewBatchGroupOutcome {
                reason: None,
                record_ids: relationship.record_ids(),
                status: ReviewBatchGroupStatus::AlreadyCommitted,
            });
        }
        let confirmed_record_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM external_records \
             JOIN accounts ON accounts.id = external_records.account_id \
             WHERE external_records.id IN (?1, ?2) AND accounts.status = 'confirmed'",
            params![relationship.first_record_id, relationship.second_record_id],
            |row| row.get(0),
        )?;
        if confirmed_record_count != 2 {
            return Ok(ReviewBatchGroupOutcome {
                reason: Some("account_confirmation_required".to_owned()),
                record_ids: relationship.record_ids(),
                status: ReviewBatchGroupStatus::StillNeedsReview,
            });
        }
        let event_id = new_database_id("event");
        transaction.execute(
            "INSERT INTO ledger_events( \
               id, event_type, event_class, event_date, status, commit_idempotency_key \
             ) VALUES (?1, ?2, 'posting', ?3, 'pending', ?4)",
            params![event_id, event.event_type, event.event_date, commit_key],
        )?;
        {
            let mut insert_leg = transaction.prepare(
                "INSERT INTO ledger_legs( \
                   id, ledger_event_id, account_id, instrument_id, amount_value, currency \
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for (index, leg) in event.legs.iter().enumerate() {
                insert_leg.execute(params![
                    format!("{event_id}:leg:{}", index + 1),
                    event_id,
                    leg.account_id,
                    leg.instrument_id,
                    leg.amount_value,
                    leg.currency,
                ])?;
            }
        }
        {
            let mut insert_edge = transaction.prepare(
                "INSERT INTO match_edges( \
                   external_record_id, ledger_event_id, allocation_value, unit, \
                   review_status, unmatched_remainder_value \
                 ) VALUES (?1, ?2, ?3, ?4, 'confirmed', '0')",
            )?;
            for record_id in &event.source_record_ids {
                insert_edge.execute(params![
                    record_id,
                    event_id,
                    relationship.allocation_value,
                    relationship.unit,
                ])?;
            }
        }
        transaction.execute(
            "UPDATE external_records SET status = 'committed' WHERE id IN (?1, ?2)",
            params![relationship.first_record_id, relationship.second_record_id],
        )?;
        transaction.execute(
            "UPDATE review_items SET status = 'resolved' WHERE id IN (?1, ?2)",
            params![
                relationship.first_review_item_id,
                relationship.second_review_item_id
            ],
        )?;
        transaction.execute(
            "UPDATE review_relationships SET status = 'committed' WHERE id = ?1",
            [&group.relationship_id],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) VALUES (?1, 'ledger_event', ?2, 'review_batch_committed', 'user', \
                       'confirmed_relationship', ?3, ?4)",
            params![
                new_audit_id(),
                event_id,
                serde_json::to_string(&event.source_record_ids)?,
                REVIEW_POLICY_VERSION,
            ],
        )?;
        transaction.execute(
            "UPDATE ledger_events SET status = 'committed' WHERE id = ?1",
            [&event_id],
        )?;
        transaction.commit()?;
        Ok(ReviewBatchGroupOutcome {
            reason: None,
            record_ids: relationship.record_ids(),
            status: ReviewBatchGroupStatus::Committed,
        })
    }

    pub(crate) fn committed_review_event_for_reversal(
        &self,
        event_id: &str,
    ) -> StoreResult<Option<CorePreparedReviewEvent>> {
        let event = self
            .connection
            .query_row(
                "SELECT event_type, event_class, event_date \
                 FROM ledger_events \
                 WHERE id = ?1 AND status = 'committed' \
                   AND event_class = 'posting' \
                   AND event_type IN ('same_currency_transfer', 'credit_card_repayment') \
                   AND NOT EXISTS( \
                     SELECT 1 FROM ledger_events reversal \
                     WHERE reversal.reverses_event_id = ledger_events.id \
                   )",
                [event_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((event_type, event_class, event_date)) = event else {
            return Ok(None);
        };
        let mut leg_statement = self.connection.prepare(
            "SELECT account_id, amount_value, currency, instrument_id \
             FROM ledger_legs WHERE ledger_event_id = ?1 \
             ORDER BY id",
        )?;
        let legs = leg_statement
            .query_map([event_id], |row| {
                Ok(CoreReviewLeg {
                    account_id: row.get(0)?,
                    amount_value: row.get(1)?,
                    currency: row.get(2)?,
                    instrument_id: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut record_statement = self.connection.prepare(
            "SELECT external_record_id FROM match_edges \
             WHERE ledger_event_id = ?1 ORDER BY external_record_id",
        )?;
        let source_record_ids = record_statement
            .query_map([event_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let event = CorePreparedReviewEvent {
            event_class,
            event_date,
            event_type,
            legs,
            source_record_ids,
            spending: false,
        };
        Ok(valid_core_review_event(&event).then_some(event))
    }

    pub(crate) fn persist_review_reversal(
        &mut self,
        original_event_id: &str,
        reversal: &CorePreparedReversalEvent,
    ) -> StoreResult<Option<UndoOutcome>> {
        if !valid_core_review_reversal(reversal) {
            return Ok(None);
        }
        let transaction = self.connection.transaction()?;
        let original = transaction
            .query_row(
                "SELECT event_type, event_date FROM ledger_events \
                 WHERE id = ?1 AND status = 'committed' AND event_class = 'posting'",
                [original_event_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((original_event_type, original_event_date)) = original else {
            return Ok(None);
        };
        if reversal.event_type != format!("{original_event_type}_reversal")
            || reversal.event_date != original_event_date
        {
            return Ok(None);
        }
        let original_legs = {
            let mut statement = transaction.prepare(
                "SELECT account_id, amount_value, currency, instrument_id \
                 FROM ledger_legs \
                 WHERE ledger_event_id = ?1 \
                   AND amount_value IS NOT NULL \
                   AND currency IS NOT NULL \
                 ORDER BY id",
            )?;
            statement
                .query_map([original_event_id], |row| {
                    Ok(CoreReviewLeg {
                        account_id: row.get(0)?,
                        amount_value: row.get(1)?,
                        currency: row.get(2)?,
                        instrument_id: row.get(3)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        if !matches_inverted_original_legs(&original_legs, &reversal.legs) {
            return Ok(None);
        }
        let existing = transaction
            .query_row(
                "SELECT id FROM ledger_events WHERE reverses_event_id = ?1",
                [original_event_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(event_id) = existing {
            return Ok(Some(UndoOutcome {
                event_id,
                status: UndoStatus::AlreadyUndone,
            }));
        }
        let event_id = new_database_id("event");
        transaction.execute(
            "INSERT INTO ledger_events( \
               id, event_type, event_class, event_date, status, commit_idempotency_key, reverses_event_id \
             ) VALUES (?1, ?2, 'posting', ?3, 'pending', ?4, ?5)",
            params![
                event_id,
                reversal.event_type,
                reversal.event_date,
                format!("review-undo:{original_event_id}"),
                original_event_id,
            ],
        )?;
        {
            let mut insert_leg = transaction.prepare(
                "INSERT INTO ledger_legs( \
                   id, ledger_event_id, account_id, instrument_id, amount_value, currency \
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for (index, leg) in reversal.legs.iter().enumerate() {
                insert_leg.execute(params![
                    format!("{event_id}:leg:{}", index + 1),
                    event_id,
                    leg.account_id,
                    leg.instrument_id,
                    leg.amount_value,
                    leg.currency,
                ])?;
            }
        }
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) VALUES (?1, 'ledger_event', ?2, 'ledger_event_undone', 'user', \
                       'undo', ?3, ?4)",
            params![
                new_audit_id(),
                event_id,
                original_event_id,
                REVIEW_POLICY_VERSION,
            ],
        )?;
        transaction.execute(
            "UPDATE ledger_events SET status = 'committed' WHERE id = ?1",
            [&event_id],
        )?;
        transaction.commit()?;
        Ok(Some(UndoOutcome {
            event_id,
            status: UndoStatus::Undone,
        }))
    }

    pub(crate) fn fail_review_batch(
        &mut self,
        claimed: &ClaimedReviewBatch,
        error_code: &str,
    ) -> StoreResult<ReviewJobSummary> {
        if error_code.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "review error code is required",
            )
            .into());
        }
        let error_json = serde_json::to_string(&serde_json::json!({ "errorCode": error_code }))?;
        let changed = self.connection.execute(
            "UPDATE jobs SET status = 'failed', error_json = ?1, lease_owner = NULL, \
                     lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?2 AND job_type = ?3 AND status = 'running' AND lease_owner = ?4",
            params![
                error_json,
                claimed.job_id,
                COMMIT_REVIEW_BATCH_JOB_TYPE,
                claimed.lease_owner
            ],
        )?;
        if changed != 1 {
            return Err(
                io::Error::new(io::ErrorKind::InvalidData, "review job lease changed").into(),
            );
        }
        self.review_job(&claimed.job_id)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "failed review job was not persisted",
            )
            .into()
        })
    }

    pub(crate) fn finish_review_batch(
        &mut self,
        claimed: &ClaimedReviewBatch,
        outcomes: &[ReviewBatchGroupOutcome],
    ) -> StoreResult<ReviewJobSummary> {
        let result_json = serde_json::to_string(&serde_json::json!({ "outcomes": outcomes }))?;
        let changed = self.connection.execute(
            "UPDATE jobs SET status = 'succeeded', result_json = ?1, lease_owner = NULL, \
                     lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?2 AND job_type = ?3 AND status = 'running' AND lease_owner = ?4",
            params![
                result_json,
                claimed.job_id,
                COMMIT_REVIEW_BATCH_JOB_TYPE,
                claimed.lease_owner
            ],
        )?;
        if changed != 1 {
            return Err(
                io::Error::new(io::ErrorKind::InvalidData, "review job lease changed").into(),
            );
        }
        self.review_job(&claimed.job_id)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "completed review job was not persisted",
            )
            .into()
        })
    }

    fn open_review_core_record(
        &self,
        review_item_id: &str,
        expected_record_version: i64,
    ) -> StoreResult<Option<(String, Option<String>, CoreReviewRecord)>> {
        let row = self
            .connection
            .query_row(
                "SELECT external_records.id, external_records.event_type, external_records.id, \
                        external_records.account_id, accounts.account_type, external_records.currency, \
                        external_records.posted_on, external_records.account_balance_delta, instruments.id \
                 FROM review_items \
                 JOIN external_records ON external_records.id = review_items.external_record_id \
                 JOIN accounts ON accounts.id = external_records.account_id \
                 JOIN instruments ON instruments.currency = external_records.currency \
                    AND instruments.instrument_type = 'fiat_currency' \
                 WHERE review_items.id = ?1 AND review_items.status = 'open' \
                   AND external_records.version = ?2 \
                   AND external_records.status IN ('staged', 'review')",
                params![review_item_id, expected_record_version],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        CoreReviewRecord {
                            id: row.get(2)?,
                            account_id: row.get(3)?,
                            account_type: row.get(4)?,
                            currency: row.get(5)?,
                            posted_on: row.get(6)?,
                            account_balance_delta: row.get(7)?,
                            instrument_id: row.get(8)?,
                        },
                    ))
                },
            )
            .optional()?;
        Ok(row)
    }

    fn review_item_record_id(&self, review_item_id: &str) -> StoreResult<Option<String>> {
        Ok(self
            .connection
            .query_row(
                "SELECT external_record_id FROM review_items WHERE id = ?1",
                [review_item_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?)
    }

    fn relationships_for_review_item(
        &self,
        review_item_id: &str,
    ) -> StoreResult<Vec<StoredReviewRelationship>> {
        let mut statement = self.connection.prepare(
            "SELECT id, event_type, first_external_record_id, second_external_record_id, \
                    first_review_item_id, second_review_item_id, allocation_value, unit, status \
             FROM review_relationships \
             WHERE first_review_item_id = ?1 OR second_review_item_id = ?1 \
             ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([review_item_id], |row| {
            Ok(StoredReviewRelationship {
                id: row.get(0)?,
                event_type: row.get(1)?,
                first_record_id: row.get(2)?,
                second_record_id: row.get(3)?,
                first_review_item_id: row.get(4)?,
                second_review_item_id: row.get(5)?,
                allocation_value: row.get(6)?,
                unit: row.get(7)?,
                status: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn core_record_by_id(&self, record_id: &str) -> StoreResult<Option<CoreReviewRecord>> {
        let row = self
            .connection
            .query_row(
                "SELECT external_records.id, external_records.account_id, accounts.account_type, \
                        external_records.currency, external_records.posted_on, \
                        external_records.account_balance_delta, instruments.id \
                 FROM external_records \
                 JOIN accounts ON accounts.id = external_records.account_id \
                 JOIN instruments ON instruments.currency = external_records.currency \
                    AND instruments.instrument_type = 'fiat_currency' \
                 WHERE external_records.id = ?1 \
                   AND external_records.status IN ('staged', 'review')",
                [record_id],
                core_review_record_from_row,
            )
            .optional()?;
        Ok(row)
    }

    fn reconcile_files(&mut self) -> StoreResult<()> {
        let documents = {
            let mut statement = self.connection.prepare(
                "SELECT id, file_sha256, encrypted_locator, file_state \
                 FROM source_documents \
                 WHERE encrypted_locator IS NOT NULL",
            )?;
            let rows = statement.query_map([], |row| {
                Ok(ExistingDocument {
                    document_id: row.get(0)?,
                    encrypted_locator: row.get(2)?,
                    file_sha256: row.get(1)?,
                    file_state: row.get(3)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let mut referenced = HashSet::new();
        for document in documents {
            let Some(locator) = document.encrypted_locator.as_deref() else {
                continue;
            };
            referenced.insert(locator.to_owned());
            if document.file_state == "available"
                && !self
                    .files
                    .verifies(&self.master_key, locator, &document.file_sha256)?
            {
                mark_missing(&mut self.connection, &document, &document.file_sha256)?;
            }
        }
        self.files.remove_unreferenced(&referenced)?;
        Ok(())
    }
}

fn parse_job_claimed(
    transaction: &Transaction<'_>,
    claim: &ParseDocumentClaim,
) -> rusqlite::Result<bool> {
    let input_json = transaction
        .query_row(
            "SELECT input_json FROM jobs WHERE id = ?1 AND job_type = ?2 \
             AND related_source_document_id = ?3 AND status = 'running' \
             AND lease_owner = ?4 AND lease_until > CURRENT_TIMESTAMP",
            params![
                claim.job_id,
                PARSE_DOCUMENT_JOB_TYPE,
                claim.document_id,
                claim.claim_token,
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(input_json
        .and_then(|input_json| serde_json::from_str::<ParseDocumentJobInput>(&input_json).ok())
        .is_some_and(|input| input.logical_run_key == claim.logical_run_key))
}

fn parse_job_update(changed: usize) -> StoreResult<()> {
    if changed != 1 {
        return Err(io::Error::other("parse job is no longer claimed").into());
    }
    Ok(())
}

fn enqueue_parse_document(
    transaction: &Transaction<'_>,
    document_id: &str,
    logical_run_key: &str,
) -> rusqlite::Result<()> {
    if document_id.is_empty() || logical_run_key.is_empty() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    transaction.execute(
        "INSERT INTO jobs(id, job_type, status, input_json, related_source_document_id) \
         VALUES (?1, ?2, 'queued', ?3, ?4)",
        params![
            new_database_id("job"),
            PARSE_DOCUMENT_JOB_TYPE,
            serde_json::to_string(&ParseDocumentJobInput {
                document_id: document_id.to_owned(),
                logical_run_key: logical_run_key.to_owned(),
            })
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
            document_id,
        ],
    )?;
    Ok(())
}

fn enqueue_reconcile_document(
    transaction: &Transaction<'_>,
    document_id: &str,
) -> rusqlite::Result<()> {
    let exists: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM source_documents WHERE id = ?1)",
        [document_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    let status = transaction
        .query_row(
            "SELECT status FROM jobs \
             WHERE related_source_document_id = ?1 AND job_type = ?2 ORDER BY id LIMIT 1",
            params![document_id, RECONCILE_DOCUMENT_JOB_TYPE],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match status.as_deref() {
        None => {
            transaction.execute(
                "INSERT INTO jobs(id, job_type, status, input_json, related_source_document_id) \
                 VALUES (?1, ?2, 'queued', ?3, ?4)",
                params![
                    new_database_id("job"),
                    RECONCILE_DOCUMENT_JOB_TYPE,
                    serde_json::json!({ "documentId": document_id }).to_string(),
                    document_id,
                ],
            )?;
        }
        Some("queued" | "running") => {}
        Some(_) => {
            transaction.execute(
                "UPDATE jobs SET status = 'queued', attempts = 0, result_json = NULL, \
                         error_json = NULL, blocked_reason = NULL, lease_owner = NULL, \
                         lease_until = NULL, finished_at = NULL, updated_at = CURRENT_TIMESTAMP \
                 WHERE related_source_document_id = ?1 AND job_type = ?2 \
                   AND status NOT IN ('queued', 'running')",
                params![document_id, RECONCILE_DOCUMENT_JOB_TYPE],
            )?;
        }
    }
    Ok(())
}

mod accounts;
#[cfg(test)]
mod accounts_tests;
mod audit;
#[cfg(test)]
mod audit_tests;
#[cfg(test)]
mod database_test_support;
mod gmail;
#[cfg(test)]
mod gmail_tests;
#[cfg(test)]
mod hardening_tests;
pub(crate) mod imports;
pub(crate) mod intake;
#[cfg(test)]
mod intake_migration_tests;
#[cfg(test)]
pub(crate) mod intake_test_support;
mod migrations;
mod parse_jobs;
#[cfg(test)]
mod reparse_tests;
pub(crate) mod restore_decisions;
mod review_records;
#[cfg(test)]
mod review_records_tests;
mod rows;
pub(crate) mod tasks;
pub(crate) mod tasks_reconcile;
#[cfg(test)]
mod tasks_test_support;
#[cfg(test)]
mod tests;
mod validation;

pub(crate) use audit::*;
#[cfg(test)]
pub(crate) use database_test_support::StructuredParseTestState;
pub(crate) use gmail::*;
use imports::*;
use migrations::*;
use rows::*;
use validation::*;
