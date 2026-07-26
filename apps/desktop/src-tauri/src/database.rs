use crate::{
    vault::{FileVault, PreparedSource, StoredFile},
    viewer::validate_image_container,
};
use hkdf::Hkdf;
use rand::{RngCore, rngs::OsRng};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Row, Transaction, TransactionBehavior, params,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    error::Error,
    fs, io,
    path::Path,
};
use zeroize::Zeroizing;

const KEY_LEN: usize = 32;
pub(crate) const DATABASE_FILE_NAME: &str = "finance.sqlite";
const DATABASE_KEY_CONTEXT: &[u8] = b"cancan:database:v1";
struct Migration {
    version: i64,
    sql: &'static str,
    foreign_keys_off: bool,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../../../../packages/db/migrations/0001_synthetic_core.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 2,
        sql: include_str!("../../../../packages/db/migrations/0002_vault_manual_import.sql"),
        foreign_keys_off: false,
    },
    Migration {
        version: 3,
        sql: include_str!(
            "../../../../packages/db/migrations/0003_source_document_pending_identity.sql"
        ),
        foreign_keys_off: true,
    },
    Migration {
        version: 4,
        sql: include_str!(
            "../../../../packages/db/migrations/0004_source_document_pending_source.sql"
        ),
        foreign_keys_off: true,
    },
    Migration {
        version: 5,
        sql: include_str!(
            "../../../../packages/db/migrations/0005_money_source_statement_password.sql"
        ),
        foreign_keys_off: false,
    },
    Migration {
        version: 6,
        sql: include_str!("../../../../packages/db/migrations/0006_review_ledger.sql"),
        foreign_keys_off: true,
    },
    Migration {
        version: 7,
        sql: include_str!("../../../../packages/db/migrations/0007_local_inbox.sql"),
        foreign_keys_off: false,
    },
];

const REVIEW_POLICY_VERSION: &str = "review-ledger-v1";
const ACCOUNT_CONFIRMATION_POLICY_VERSION: &str = "account-confirmation-v1";
const COMMIT_REVIEW_BATCH_JOB_TYPE: &str = "commit_review_batch";
const PARSE_DOCUMENT_JOB_TYPE: &str = "parse_document";
const RECONCILE_DOCUMENT_JOB_TYPE: &str = "reconcile_document";
const SOURCE_DOCUMENT_INGEST_JOB_TYPE: &str = "source_document_ingest";
const COMMIT_REVIEW_BATCH_LEASE_SECONDS: i64 = 300;
const MAX_SUPPORTED_RELATIONSHIP_WINDOW_DAYS: i64 = 7;
const MAX_PERSISTED_PARSE_JSON_BYTES: usize = 16 * 1024;

type StoreResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

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
    pub(crate) primary_record_id: String,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountConfirmationCandidate {
    pub(crate) account_id: String,
    pub(crate) account_type: String,
    pub(crate) currency: Option<String>,
    pub(crate) display_name: String,
    pub(crate) masked_identifier: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountConfirmationPrompt {
    pub(crate) candidate_accounts: Vec<AccountConfirmationCandidate>,
    pub(crate) display_name: String,
    pub(crate) money_source_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AccountConfirmationStatus {
    AlreadyConfirmed,
    Confirmed,
    Conflict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountConfirmationOutcome {
    pub(crate) status: AccountConfirmationStatus,
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

#[cfg(test)]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StructuredParseTestState {
    pub(crate) balance_snapshots_without_amount: i64,
    pub(crate) ledger_events: i64,
    pub(crate) open_review_items: i64,
    pub(crate) parse_runs: i64,
    pub(crate) reconcile_status: Option<String>,
    pub(crate) records: i64,
    pub(crate) staged_records: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StatementCoveragePolicy<'a> {
    pub(crate) cadence_months: u32,
    pub(crate) document_type: &'a str,
    pub(crate) grace_days: i64,
    pub(crate) provider_key: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StatementCoveragePromptStatus {
    ConfirmedMissing,
    LikelyMissing,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatementCoveragePrompt {
    pub(crate) account_id: String,
    pub(crate) document_type: String,
    pub(crate) money_source_id: String,
    pub(crate) statement_period_from: String,
    pub(crate) statement_period_to: String,
    pub(crate) status: StatementCoveragePromptStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatementCoverageDecision {
    NotExpected,
    RemindLater,
}

pub(crate) struct StatementCoverageDecisionInput<'a> {
    pub(crate) account_id: &'a str,
    pub(crate) audit_id: &'a str,
    pub(crate) decision: StatementCoverageDecision,
    pub(crate) document_type: &'a str,
    pub(crate) money_source_id: &'a str,
    pub(crate) remind_after: Option<&'a str>,
    pub(crate) statement_period_from: &'a str,
    pub(crate) statement_period_to: &'a str,
}

#[derive(Debug)]
struct ExistingDocument {
    document_id: String,
    encrypted_locator: Option<String>,
    file_sha256: String,
    file_state: String,
}

#[derive(Clone, Debug)]
struct CoveragePeriod {
    statement_period_from: String,
    statement_period_to: String,
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
        store.recover_expired_review_jobs()?;
        Ok(store)
    }

    pub(crate) fn master_key(&self) -> &[u8; KEY_LEN] {
        &self.master_key
    }

    pub fn register_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
        restore_deleted_document_id: Option<&str>,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        validate_import(input)?;
        let source = FileVault::prepare(input.source_path)?;
        self.register_prepared_import(input, source, restore_deleted_document_id)
    }

    pub(crate) fn register_captured_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
        captured_bytes: Zeroizing<Vec<u8>>,
        restore_deleted_document_id: Option<&str>,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        validate_import(input)?;
        validate_captured_container(input.mime_type, &captured_bytes)?;
        let source = FileVault::prepare_bytes(captured_bytes)?;
        self.register_prepared_import(input, source, restore_deleted_document_id)
    }

    fn register_prepared_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
        source: PreparedSource,
        restore_deleted_document_id: Option<&str>,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        if matches!(input.mime_type, "image/png" | "image/jpeg") {
            validate_image_container(source.plaintext(), input.mime_type)?;
        }
        let existing = find_exact_document(&self.connection, source.file_sha256())?;
        match (existing.as_ref(), restore_deleted_document_id) {
            (Some(existing), None) if existing.file_state == "deleted" => {
                return Ok(SourceDocumentImportOutcome {
                    document_id: existing.document_id.clone(),
                    status: SourceDocumentImportStatus::RestoreConfirmationRequired,
                });
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
        let replace_existing = existing
            .as_ref()
            .is_some_and(|document| document.file_state != "available");
        let stored = match self
            .files
            .store_prepared(&self.master_key, &source, replace_existing)
        {
            Ok(stored) => stored,
            Err(error)
                if existing
                    .as_ref()
                    .is_some_and(|document| document.file_state == "available")
                    && matches!(
                        error.kind(),
                        io::ErrorKind::InvalidData | io::ErrorKind::NotFound
                    ) =>
            {
                let document = existing.as_ref().expect("guarded existing document");
                mark_missing(&mut self.connection, document, source.file_sha256())?;
                self.files.store_prepared(&self.master_key, &source, true)?
            }
            Err(error) => return Err(error.into()),
        };

        match persist_import(&mut self.connection, input, &stored) {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                if stored.created {
                    let _ = self.files.remove(&stored.encrypted_locator);
                }
                Err(error.into())
            }
        }
    }

    pub(crate) fn deleted_source_hashes(&self) -> StoreResult<BTreeSet<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT file_sha256 FROM source_documents WHERE file_state = 'deleted'")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub(crate) fn enqueue_source_document_pipeline(
        &mut self,
        document_id: &str,
    ) -> StoreResult<()> {
        if document_id.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "source document id must not be empty",
            )
            .into());
        }
        let transaction = self.connection.transaction()?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM source_documents WHERE id = ?1)",
            [document_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "source document not found").into(),
            );
        }
        let ingest_exists: bool = transaction.query_row(
            "SELECT EXISTS( \
               SELECT 1 FROM jobs \
               WHERE related_source_document_id = ?1 \
                 AND job_type = ?2 \
             )",
            params![document_id, SOURCE_DOCUMENT_INGEST_JOB_TYPE],
            |row| row.get(0),
        )?;
        let input_json = serde_json::json!({ "documentId": document_id }).to_string();
        if !ingest_exists {
            transaction.execute(
                "INSERT INTO jobs( \
                   id, job_type, status, input_json, result_json, related_source_document_id, finished_at \
                 ) VALUES (?1, ?2, 'succeeded', ?3, ?4, ?5, CURRENT_TIMESTAMP)",
                params![
                    new_database_id("job"),
                    SOURCE_DOCUMENT_INGEST_JOB_TYPE,
                    input_json,
                    serde_json::json!({ "nextJobType": PARSE_DOCUMENT_JOB_TYPE }).to_string(),
                    document_id,
                ],
            )?;
        }
        let parse_exists: bool = transaction.query_row(
            "SELECT EXISTS( \
               SELECT 1 FROM jobs \
               WHERE related_source_document_id = ?1 \
                 AND job_type = ?2 \
             )",
            params![document_id, PARSE_DOCUMENT_JOB_TYPE],
            |row| row.get(0),
        )?;
        if !parse_exists {
            transaction.execute(
                "INSERT INTO jobs(id, job_type, status, input_json, related_source_document_id) \
                 VALUES (?1, ?2, 'queued', ?3, ?4)",
                params![
                    new_database_id("job"),
                    PARSE_DOCUMENT_JOB_TYPE,
                    input_json,
                    document_id
                ],
            )?;
        } else {
            transaction.execute(
                "UPDATE jobs \
                 SET status = 'queued', result_json = NULL, error_json = NULL, \
                     blocked_reason = NULL, lease_owner = NULL, lease_until = NULL, \
                     finished_at = NULL, updated_at = CURRENT_TIMESTAMP \
                 WHERE related_source_document_id = ?1 AND job_type = ?2 \
                   AND status = 'failed' AND attempts < max_attempts",
                params![document_id, PARSE_DOCUMENT_JOB_TYPE],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn queued_parse_document_ids(&self) -> StoreResult<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT related_source_document_id FROM jobs \
             WHERE job_type = ?1 AND status = 'queued' \
             ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([PARSE_DOCUMENT_JOB_TYPE], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub(crate) fn start_parse_document(&mut self, document_id: &str) -> StoreResult<bool> {
        let changed = self.connection.execute(
            "UPDATE jobs \
             SET status = 'running', attempts = attempts + 1, \
                 lease_owner = 'local-inbox', lease_until = datetime('now', '+300 seconds'), \
                 started_at = COALESCE(started_at, CURRENT_TIMESTAMP), updated_at = CURRENT_TIMESTAMP \
             WHERE related_source_document_id = ?1 AND job_type = ?2 \
               AND status = 'queued' AND attempts < max_attempts",
            params![document_id, PARSE_DOCUMENT_JOB_TYPE],
        )?;
        if changed == 0 {
            self.connection.execute(
                "UPDATE jobs SET status = 'failed', blocked_reason = 'retry_limit_reached', \
                         finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
                 WHERE related_source_document_id = ?1 AND job_type = ?2 \
                   AND status = 'queued' AND attempts >= max_attempts",
                params![document_id, PARSE_DOCUMENT_JOB_TYPE],
            )?;
        }
        Ok(changed == 1)
    }

    pub(crate) fn finish_parse_document(
        &mut self,
        document_id: &str,
        outcome: &SourceDocumentRoutingOutcome,
    ) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        match outcome.status {
            SourceDocumentRoutingStatus::Routed => {
                transaction.execute(
                    "UPDATE jobs SET status = 'succeeded', result_json = ?1, lease_owner = NULL, \
                         lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
                     WHERE related_source_document_id = ?2 AND job_type = ?3 \
                       AND status IN ('queued', 'running')",
                    params![
                        serde_json::json!({ "nextJobType": RECONCILE_DOCUMENT_JOB_TYPE }).to_string(),
                        document_id,
                        PARSE_DOCUMENT_JOB_TYPE,
                    ],
                )?;
                enqueue_reconcile_document(&transaction, document_id)?;
            }
            SourceDocumentRoutingStatus::NeedsAttention => {
                transaction.execute(
                    "UPDATE jobs SET status = 'blocked', blocked_reason = ?1, lease_owner = NULL, \
                         lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
                     WHERE related_source_document_id = ?2 AND job_type = ?3 \
                       AND status IN ('queued', 'running')",
                    params![
                        outcome.reason.unwrap_or("classification_uncertain"),
                        document_id,
                        PARSE_DOCUMENT_JOB_TYPE,
                    ],
                )?;
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
                 lease_until = datetime('now', '+300 seconds'), \
                 started_at = COALESCE(started_at, CURRENT_TIMESTAMP), updated_at = CURRENT_TIMESTAMP \
             WHERE related_source_document_id = ?1 AND job_type = ?2 \
               AND status = 'queued' AND attempts < max_attempts",
            params![document_id, RECONCILE_DOCUMENT_JOB_TYPE],
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

    pub(crate) fn block_parse_document(
        &mut self,
        document_id: &str,
        reason: &'static str,
    ) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs SET status = 'blocked', blocked_reason = ?1, lease_owner = NULL, \
                     lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE related_source_document_id = ?2 AND job_type = ?3 \
               AND status IN ('queued', 'running')",
            params![reason, document_id, PARSE_DOCUMENT_JOB_TYPE],
        )?;
        Ok(())
    }

    pub(crate) fn fail_parse_document(
        &mut self,
        document_id: &str,
        reason: &'static str,
    ) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs SET status = 'failed', blocked_reason = ?1, lease_owner = NULL, \
                     lease_until = NULL, finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE related_source_document_id = ?2 AND job_type = ?3 \
               AND status = 'running'",
            params![reason, document_id, PARSE_DOCUMENT_JOB_TYPE],
        )?;
        Ok(())
    }

    pub(crate) fn statement_coverage_today(&self) -> StoreResult<String> {
        Ok(self
            .connection
            .query_row("SELECT date('now')", [], |row| row.get(0))?)
    }

    pub(crate) fn list_statement_coverage_prompts(
        &self,
        policies: &[StatementCoveragePolicy<'_>],
        today: &str,
    ) -> StoreResult<Vec<StatementCoveragePrompt>> {
        if !valid_iso_date(today)
            || policies
                .iter()
                .any(|policy| policy.cadence_months == 0 || policy.grace_days < 0)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid statement coverage policy or date",
            )
            .into());
        }
        let mut statement = self.connection.prepare(
            "SELECT money_sources.provider_key, source_documents.money_source_id, \
                    source_document_accounts.account_id, source_documents.document_type, \
                    source_documents.statement_period_from, source_documents.statement_period_to \
             FROM source_documents \
             JOIN source_document_accounts \
               ON source_document_accounts.source_document_id = source_documents.id \
             JOIN money_sources ON money_sources.id = source_documents.money_source_id \
             WHERE source_documents.document_type IS NOT NULL \
               AND source_documents.statement_period_from IS NOT NULL \
               AND source_documents.statement_period_to IS NOT NULL",
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
        let mut periods = BTreeMap::<(String, String, String, String), Vec<CoveragePeriod>>::new();
        for row in rows {
            let (provider_key, money_source_id, account_id, document_type, from, to) = row?;
            if policies.iter().any(|policy| {
                policy.provider_key == provider_key && policy.document_type == document_type
            }) {
                periods
                    .entry((provider_key, money_source_id, account_id, document_type))
                    .or_default()
                    .push(CoveragePeriod {
                        statement_period_from: from,
                        statement_period_to: to,
                    });
            }
        }
        let mut prompts = Vec::new();
        for ((provider_key, money_source_id, account_id, document_type), periods) in &mut periods {
            let policy = policies
                .iter()
                .find(|policy| {
                    policy.provider_key == provider_key && policy.document_type == document_type
                })
                .expect("policy selected with the same provider and document type");
            periods.sort_by(|left, right| {
                left.statement_period_from
                    .cmp(&right.statement_period_from)
                    .then(left.statement_period_to.cmp(&right.statement_period_to))
            });
            periods.dedup_by(|left, right| {
                left.statement_period_from == right.statement_period_from
                    && left.statement_period_to == right.statement_period_to
            });
            for pair in periods.windows(2) {
                let previous = &pair[0];
                let next = &pair[1];
                let previous_period_ends_month = is_month_end_iso(&previous.statement_period_to)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                    })?;
                let mut expected_from = add_months_iso(
                    &previous.statement_period_from,
                    policy.cadence_months,
                    false,
                )
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                })?;
                let mut expected_to = add_months_iso(
                    &previous.statement_period_to,
                    policy.cadence_months,
                    previous_period_ends_month,
                )
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                })?;
                while expected_from < next.statement_period_from {
                    if !self.statement_coverage_is_suppressed(
                        money_source_id,
                        account_id,
                        document_type,
                        &expected_from,
                        &expected_to,
                        today,
                    )? {
                        prompts.push(StatementCoveragePrompt {
                            account_id: account_id.clone(),
                            document_type: document_type.clone(),
                            money_source_id: money_source_id.clone(),
                            statement_period_from: expected_from.clone(),
                            statement_period_to: expected_to.clone(),
                            status: StatementCoveragePromptStatus::ConfirmedMissing,
                        });
                    }
                    expected_from = add_months_iso(&expected_from, policy.cadence_months, false)
                        .ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                        })?;
                    expected_to = add_months_iso(
                        &expected_to,
                        policy.cadence_months,
                        previous_period_ends_month,
                    )
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                    })?;
                }
            }
            if periods.len() >= 2 {
                let latest = periods.last().expect("length checked");
                let latest_period_ends_month = is_month_end_iso(&latest.statement_period_to)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                    })?;
                let expected_from =
                    add_months_iso(&latest.statement_period_from, policy.cadence_months, false)
                        .ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                        })?;
                let expected_to = add_months_iso(
                    &latest.statement_period_to,
                    policy.cadence_months,
                    latest_period_ends_month,
                )
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid statement period")
                })?;
                let grace_end = self.add_days_iso(&expected_to, policy.grace_days)?;
                if today > grace_end.as_str()
                    && !self.statement_coverage_is_suppressed(
                        money_source_id,
                        account_id,
                        document_type,
                        &expected_from,
                        &expected_to,
                        today,
                    )?
                {
                    prompts.push(StatementCoveragePrompt {
                        account_id: account_id.clone(),
                        document_type: document_type.clone(),
                        money_source_id: money_source_id.clone(),
                        statement_period_from: expected_from,
                        statement_period_to: expected_to,
                        status: StatementCoveragePromptStatus::LikelyMissing,
                    });
                }
            }
        }
        Ok(prompts)
    }

    pub(crate) fn record_statement_coverage_decision(
        &mut self,
        input: &StatementCoverageDecisionInput<'_>,
    ) -> StoreResult<()> {
        if input.audit_id.is_empty()
            || input.money_source_id.is_empty()
            || input.account_id.is_empty()
            || input.document_type.is_empty()
            || !valid_iso_date(input.statement_period_from)
            || !valid_iso_date(input.statement_period_to)
            || input.statement_period_from > input.statement_period_to
            || !valid_optional_date(input.remind_after)
            || matches!(input.decision, StatementCoverageDecision::NotExpected)
                && input.remind_after.is_some()
            || matches!(input.decision, StatementCoverageDecision::RemindLater)
                && input.remind_after.is_none()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid statement coverage decision",
            )
            .into());
        }
        let transaction = self.connection.transaction()?;
        let account_matches_source: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = ?1 AND money_source_id = ?2)",
            params![input.account_id, input.money_source_id],
            |row| row.get(0),
        )?;
        if !account_matches_source {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "statement coverage account does not belong to source",
            )
            .into());
        }
        let (decision, remind_after) = match input.decision {
            StatementCoverageDecision::NotExpected => ("not_expected", None),
            StatementCoverageDecision::RemindLater => {
                let remind_after = input.remind_after.expect("validated remind date");
                let future: bool = transaction.query_row(
                    "SELECT date(?1) > date('now')",
                    [remind_after],
                    |row| row.get(0),
                )?;
                if !future {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "statement coverage reminder must be in the future",
                    )
                    .into());
                }
                ("remind_later", Some(remind_after))
            }
        };
        let existing = transaction
            .query_row(
                "SELECT decision, remind_after FROM statement_coverage_decisions \
                 WHERE money_source_id = ?1 AND account_id = ?2 AND document_type = ?3 \
                   AND statement_period_from = ?4 AND statement_period_to = ?5",
                params![
                    input.money_source_id,
                    input.account_id,
                    input.document_type,
                    input.statement_period_from,
                    input.statement_period_to,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        if existing
            .as_ref()
            .is_some_and(|(existing_decision, existing_remind_after)| {
                existing_decision == decision && existing_remind_after.as_deref() == remind_after
            })
        {
            transaction.commit()?;
            return Ok(());
        }
        transaction.execute(
            "INSERT INTO statement_coverage_decisions( \
               money_source_id, account_id, document_type, statement_period_from, \
               statement_period_to, decision, remind_after \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(money_source_id, account_id, document_type, statement_period_from, statement_period_to) \
             DO UPDATE SET decision = excluded.decision, remind_after = excluded.remind_after, \
                           updated_at = CURRENT_TIMESTAMP",
            params![
                input.money_source_id,
                input.account_id,
                input.document_type,
                input.statement_period_from,
                input.statement_period_to,
                decision,
                remind_after,
            ],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES (?1, 'statement_coverage', ?2, 'statement_coverage_decided', \
                       'user', ?3, 'coverage-v1')",
            params![
                input.audit_id,
                format!(
                    "{}:{}:{}:{}:{}",
                    input.money_source_id,
                    input.account_id,
                    input.document_type,
                    input.statement_period_from,
                    input.statement_period_to,
                ),
                decision,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn statement_coverage_is_suppressed(
        &self,
        money_source_id: &str,
        account_id: &str,
        document_type: &str,
        statement_period_from: &str,
        statement_period_to: &str,
        today: &str,
    ) -> StoreResult<bool> {
        let decision = self
            .connection
            .query_row(
                "SELECT decision, remind_after FROM statement_coverage_decisions \
                 WHERE money_source_id = ?1 AND account_id = ?2 AND document_type = ?3 \
                   AND statement_period_from = ?4 AND statement_period_to = ?5",
                params![
                    money_source_id,
                    account_id,
                    document_type,
                    statement_period_from,
                    statement_period_to,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        Ok(match decision {
            Some((decision, _)) if decision == "not_expected" => true,
            Some((decision, Some(remind_after))) if decision == "remind_later" => {
                remind_after.as_str() > today
            }
            _ => false,
        })
    }

    fn add_days_iso(&self, date: &str, days: i64) -> StoreResult<String> {
        let modifier = format!("+{days} days");
        Ok(self
            .connection
            .query_row("SELECT date(?1, ?2)", params![date, modifier], |row| {
                row.get(0)
            })?)
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

    pub(crate) fn list_review_items(&self) -> StoreResult<Vec<ReviewItemSummary>> {
        let mut statement = self.connection.prepare(
            "SELECT review_items.id, external_records.id, external_records.version, \
                    review_items.reason_code, external_records.amount_value, \
                    external_records.currency, external_records.event_type, \
                    external_records.posted_on, COALESCE(accounts.display_name, 'Unassigned') \
             FROM review_items \
             JOIN external_records ON external_records.id = review_items.external_record_id \
             LEFT JOIN accounts ON accounts.id = external_records.account_id \
             WHERE review_items.status = 'open' \
               AND external_records.status IN ('staged', 'review') \
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
                        COALESCE(money_sources.display_name, 'Unassigned') \
                 FROM review_items \
                 JOIN external_records ON external_records.id = review_items.external_record_id \
                 JOIN source_documents ON source_documents.id = external_records.source_document_id \
                 LEFT JOIN accounts ON accounts.id = external_records.account_id \
                 LEFT JOIN money_sources ON money_sources.id = source_documents.money_source_id \
                 WHERE review_items.id = ?1 \
                   AND review_items.status = 'open' \
                   AND external_records.status IN ('staged', 'review')",
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

    pub(crate) fn list_account_confirmation_prompts(
        &self,
    ) -> StoreResult<Vec<AccountConfirmationPrompt>> {
        let mut statement = self.connection.prepare(
            "SELECT money_sources.id, money_sources.display_name, accounts.id, \
                    accounts.display_name, accounts.account_type, accounts.masked_identifier, \
                    accounts.currency \
             FROM accounts \
             JOIN money_sources ON money_sources.id = accounts.money_source_id \
             WHERE accounts.status = 'candidate' \
             ORDER BY money_sources.display_name, money_sources.id, accounts.display_name, accounts.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                AccountConfirmationCandidate {
                    account_id: row.get(2)?,
                    display_name: row.get(3)?,
                    account_type: row.get(4)?,
                    masked_identifier: row.get(5)?,
                    currency: row.get(6)?,
                },
            ))
        })?;
        let mut prompts: Vec<AccountConfirmationPrompt> = Vec::new();
        for row in rows {
            let (money_source_id, display_name, candidate) = row?;
            match prompts.last_mut() {
                Some(prompt) if prompt.money_source_id == money_source_id => {
                    prompt.candidate_accounts.push(candidate);
                }
                _ => prompts.push(AccountConfirmationPrompt {
                    candidate_accounts: vec![candidate],
                    display_name,
                    money_source_id,
                }),
            }
        }
        Ok(prompts)
    }

    pub(crate) fn confirm_candidate_accounts(
        &mut self,
        money_source_id: &str,
        expected_candidate_account_ids: &[String],
        audit_id: &str,
    ) -> StoreResult<AccountConfirmationOutcome> {
        let expected_ids = expected_candidate_account_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if money_source_id.is_empty()
            || audit_id.is_empty()
            || expected_ids.is_empty()
            || expected_ids.len() != expected_candidate_account_ids.len()
            || expected_ids.iter().any(|account_id| account_id.is_empty())
        {
            return Ok(AccountConfirmationOutcome {
                status: AccountConfirmationStatus::Conflict,
            });
        }

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let account_statuses = {
            let mut statement = transaction.prepare(
                "SELECT id, status FROM accounts WHERE money_source_id = ?1 ORDER BY id",
            )?;
            statement
                .query_map([money_source_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<BTreeMap<_, _>, _>>()?
        };
        let candidate_ids = account_statuses
            .iter()
            .filter_map(|(account_id, status)| {
                (status == "candidate").then_some(account_id.clone())
            })
            .collect::<BTreeSet<_>>();
        if candidate_ids == expected_ids {
            transaction.execute(
                "UPDATE accounts SET status = 'confirmed', updated_at = CURRENT_TIMESTAMP \
                 WHERE money_source_id = ?1 AND status = 'candidate'",
                [money_source_id],
            )?;
            transaction.execute(
                "INSERT INTO audit_log( \
                   id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
                 ) VALUES (?1, 'money_source', ?2, 'candidate_accounts_confirmed', 'user', \
                           'candidate_accounts_confirmed', ?3, ?4)",
                params![
                    audit_id,
                    money_source_id,
                    format!("candidate_accounts={}", expected_ids.len()),
                    ACCOUNT_CONFIRMATION_POLICY_VERSION,
                ],
            )?;
            transaction.commit()?;
            return Ok(AccountConfirmationOutcome {
                status: AccountConfirmationStatus::Confirmed,
            });
        }
        if candidate_ids.is_empty()
            && expected_ids.iter().all(|account_id| {
                account_statuses
                    .get(account_id)
                    .is_some_and(|status| status == "confirmed")
            })
        {
            return Ok(AccountConfirmationOutcome {
                status: AccountConfirmationStatus::AlreadyConfirmed,
            });
        }
        Ok(AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Conflict,
        })
    }

    pub fn source_document_input(&self, document_id: &str) -> StoreResult<SourceDocumentFileInput> {
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
        let plaintext = self
            .files
            .open_in_memory(&self.master_key, &encrypted_locator)?;
        Ok(SourceDocumentFileInput {
            file_sha256,
            mime_type,
            plaintext,
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

    pub fn apply_trusted_classification(
        &mut self,
        input: &TrustedDocumentClassification<'_>,
    ) -> StoreResult<SourceDocumentRoutingOutcome> {
        validate_classification(input)?;
        let transaction = self.connection.transaction()?;
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

        let mut source_statement = transaction
            .prepare("SELECT id FROM money_sources WHERE provider_key = ?1 ORDER BY id LIMIT 2")?;
        let source_ids = source_statement
            .query_map([input.provider_key], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(source_statement);
        if source_ids.len() != 1 {
            return Ok(needs_attention(
                input.document_id,
                if source_ids.is_empty() {
                    "money_source_not_found"
                } else {
                    "money_source_ambiguous"
                },
            ));
        }
        let money_source_id = &source_ids[0];
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

    pub(crate) fn persist_validated_structured_parse(
        &mut self,
        document_id: &str,
        input: &ValidatedStructuredParseInput,
    ) -> StoreResult<()> {
        validate_structured_parse_input(document_id, input)?;
        let transaction = self.connection.transaction()?;
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
        let existing_profile = transaction
            .query_row(
                "SELECT profile_json FROM parse_runs \
                 WHERE source_document_id = ?1 AND normalization_profile_id = ?2",
                params![document_id, input.normalization_profile_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(profile_json) = existing_profile {
            if profile_json != input.profile_json {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "normalization profile changed without a new profile id",
                )
                .into());
            }
            return Ok(());
        }

        let parse_run_id = new_database_id("parse");
        transaction.execute(
            "INSERT INTO parse_runs( \
               id, source_document_id, normalization_profile_id, profile_json, status \
             ) VALUES (?1, ?2, ?3, ?4, 'validated')",
            params![
                parse_run_id,
                document_id,
                input.normalization_profile_id,
                input.profile_json,
            ],
        )?;
        for record in &input.records {
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
            transaction.execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                   status, record_type, event_type, posted_on, amount_value, currency, \
                   account_balance_delta, raw_json, validation_json \
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'staged', ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    new_database_id("record"),
                    parse_run_id,
                    document_id,
                    record.account_id,
                    record.stable_record_key,
                    version,
                    record.record_type,
                    record.event_type,
                    record.posted_on,
                    record.amount_value,
                    record.currency,
                    record.account_balance_delta,
                    record.raw_json,
                    record.validation_json,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn structured_parse_test_state(
        &self,
        document_id: &str,
    ) -> StoreResult<StructuredParseTestState> {
        Ok(StructuredParseTestState {
            parse_runs: self.connection.query_row(
                "SELECT count(*) FROM parse_runs WHERE source_document_id = ?1",
                [document_id],
                |row| row.get(0),
            )?,
            records: self.connection.query_row(
                "SELECT count(*) FROM external_records WHERE source_document_id = ?1",
                [document_id],
                |row| row.get(0),
            )?,
            staged_records: self.connection.query_row(
                "SELECT count(*) FROM external_records \
                 WHERE source_document_id = ?1 AND status = 'staged'",
                [document_id],
                |row| row.get(0),
            )?,
            balance_snapshots_without_amount: self.connection.query_row(
                "SELECT count(*) FROM external_records \
                 WHERE source_document_id = ?1 AND record_type = 'balance' \
                   AND amount_value IS NULL",
                [document_id],
                |row| row.get(0),
            )?,
            open_review_items: self.connection.query_row(
                "SELECT count(*) FROM review_items \
                 JOIN external_records ON external_records.id = review_items.external_record_id \
                 WHERE external_records.source_document_id = ?1 \
                   AND review_items.reason_code = 'normalization_profile_unqualified' \
                   AND review_items.status = 'open'",
                [document_id],
                |row| row.get(0),
            )?,
            reconcile_status: self
                .connection
                .query_row(
                    "SELECT status FROM jobs WHERE related_source_document_id = ?1 \
                     AND job_type = 'reconcile_document'",
                    [document_id],
                    |row| row.get(0),
                )
                .optional()?,
            ledger_events: self.connection.query_row(
                "SELECT count(*) FROM ledger_events",
                [],
                |row| row.get(0),
            )?,
        })
    }

    #[cfg(test)]
    pub(crate) fn expire_reconcile_lease_for_test(&self, document_id: &str) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs SET lease_until = datetime('now', '-1 second') \
             WHERE related_source_document_id = ?1 AND job_type = 'reconcile_document'",
            [document_id],
        )?;
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
                primary_record_id: record_id,
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
            primary_record_id: record_id,
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
             WHERE status = 'running' \
               AND lease_until IS NOT NULL AND lease_until <= CURRENT_TIMESTAMP",
            [],
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

    #[cfg(test)]
    pub(crate) fn seed_money_source(
        &self,
        id: &str,
        provider_key: &str,
        display_name: &str,
        source_type: &str,
    ) -> StoreResult<()> {
        self.connection.execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES (?1, ?2, ?3, ?4)",
            params![id, provider_key, display_name, source_type],
        )?;
        Ok(())
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

fn validate_structured_parse_input(
    document_id: &str,
    input: &ValidatedStructuredParseInput,
) -> StoreResult<()> {
    if document_id.is_empty()
        || input.normalization_profile_id.is_empty()
        || input.normalization_profile_id.len() > 256
        || input.profile_json.len() > MAX_PERSISTED_PARSE_JSON_BYTES
        || input.records.is_empty()
        || input.records.len() > 1_000
        || !matches!(
            serde_json::from_str::<Value>(&input.profile_json),
            Ok(Value::Object(_))
        )
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid structured parse").into());
    }
    let mut stable_keys = HashSet::new();
    for record in &input.records {
        let valid_validation = matches!(
            serde_json::from_str::<Value>(&record.validation_json),
            Ok(Value::Object(values))
                if values.get("schemaValid") == Some(&Value::Bool(true))
                    && values.get("rawGrounded") == Some(&Value::Bool(true))
                    && values.get("deterministicValidationPassed") == Some(&Value::Bool(true))
        );
        if record.account_id.is_empty()
            || record.stable_record_key.is_empty()
            || record.stable_record_key.len() > 256
            || !stable_keys.insert(record.stable_record_key.as_str())
            || !matches!(
                record.record_type.as_str(),
                "transaction" | "balance" | "position" | "trade" | "valuation" | "fee" | "interest"
            )
            || record
                .event_type
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.len() > 128)
            || record
                .posted_on
                .as_deref()
                .is_some_and(|value| !valid_iso_date(value))
            || record
                .amount_value
                .as_deref()
                .is_some_and(|value| value.starts_with('-') || !valid_exact_decimal(value))
            || record
                .account_balance_delta
                .as_deref()
                .is_some_and(|value| !valid_exact_decimal(value))
            || record
                .currency
                .as_deref()
                .is_some_and(|value| !valid_currency(value))
            || record.raw_json.len() > MAX_PERSISTED_PARSE_JSON_BYTES
            || record.validation_json.len() > MAX_PERSISTED_PARSE_JSON_BYTES
            || !matches!(
                serde_json::from_str::<Value>(&record.raw_json),
                Ok(Value::Object(_))
            )
            || !valid_validation
        {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "invalid structured record").into(),
            );
        }
    }
    Ok(())
}

fn valid_currency(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn statement_password_state_from_row(row: &Row<'_>) -> rusqlite::Result<StatementPasswordState> {
    let status = match row.get::<_, String>(2)?.as_str() {
        "pending_delete" => StatementPasswordStatus::PendingDelete,
        "pending_save" => StatementPasswordStatus::PendingSave,
        "saved" => StatementPasswordStatus::Saved,
        _ => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid statement password status",
                )
                .into(),
            ));
        }
    };
    Ok(StatementPasswordState {
        money_source_id: row.get(0)?,
        secret_storage_key: row.get(1)?,
        status,
    })
}

fn update_statement_password_status(
    connection: &Connection,
    money_source_id: &str,
    expected_status: &str,
    next_status: &str,
) -> StoreResult<()> {
    let changed = connection.execute(
        "UPDATE statement_secret_refs SET status = ?1, updated_at = CURRENT_TIMESTAMP \
         WHERE money_source_id = ?2 AND status = ?3",
        params![next_status, money_source_id, expected_status],
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

fn source_document_from_row(row: &Row<'_>) -> rusqlite::Result<SourceDocumentView> {
    let byte_size: i64 = row.get(6)?;
    Ok(SourceDocumentView {
        byte_size: u64::try_from(byte_size).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Integer,
                Box::new(error),
            )
        })?,
        document_id: row.get(0)?,
        money_source_id: row.get(1)?,
        file_sha256: row.get(2)?,
        semantic_document_key: row.get(3)?,
        original_filename: row.get(4)?,
        mime_type: row.get(5)?,
        encrypted_locator: row.get(7)?,
        file_state: row.get(8)?,
        received_at: row.get(9)?,
    })
}

#[derive(Debug)]
struct ReviewRelationshipRecord {
    account_balance_delta: String,
    currency: String,
    event_type: Option<String>,
    record_id: String,
    review_item_id: Option<String>,
}

#[derive(Debug)]
struct StoredReviewRelationship {
    allocation_value: String,
    event_type: String,
    first_record_id: String,
    first_review_item_id: String,
    id: String,
    second_record_id: String,
    second_review_item_id: String,
    status: String,
    unit: String,
}

impl StoredReviewRelationship {
    fn record_ids(&self) -> Vec<String> {
        vec![self.first_record_id.clone(), self.second_record_id.clone()]
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredReviewJobResult {
    outcomes: Vec<ReviewBatchGroupOutcome>,
}

fn review_item_summary_from_row(row: &Row<'_>) -> rusqlite::Result<ReviewItemSummary> {
    Ok(ReviewItemSummary {
        review_item_id: row.get(0)?,
        record_id: row.get(1)?,
        record_version: row.get(2)?,
        reason_code: row.get(3)?,
        amount_value: row.get(4)?,
        currency: row.get(5)?,
        event_type: row.get(6)?,
        posted_on: row.get(7)?,
        account_label: row.get(8)?,
    })
}

fn review_item_detail_from_row(row: &Row<'_>) -> rusqlite::Result<ReviewItemDetail> {
    Ok(ReviewItemDetail {
        review_item_id: row.get(0)?,
        record_id: row.get(1)?,
        record_version: row.get(2)?,
        reason_code: row.get(3)?,
        amount_value: row.get(4)?,
        currency: row.get(5)?,
        event_type: row.get(6)?,
        posted_on: row.get(7)?,
        account_label: row.get(8)?,
        document_label: row.get(9)?,
        source_label: row.get(10)?,
    })
}

fn core_review_record_from_row(row: &Row<'_>) -> rusqlite::Result<CoreReviewRecord> {
    Ok(CoreReviewRecord {
        id: row.get(0)?,
        account_id: row.get(1)?,
        account_type: row.get(2)?,
        currency: row.get(3)?,
        posted_on: row.get(4)?,
        account_balance_delta: row.get(5)?,
        instrument_id: row.get(6)?,
    })
}

fn relationship_candidate_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<RelationshipCandidateSummary> {
    let amount_value: Option<String> = row.get(4)?;
    let account_balance_delta: Option<String> = row.get(5)?;
    Ok(RelationshipCandidateSummary {
        record_id: row.get(0)?,
        record_version: row.get(1)?,
        event_type: row.get(2)?,
        posted_on: row.get(3)?,
        amount_value: amount_value.or(account_balance_delta).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Null,
                io::Error::new(io::ErrorKind::InvalidData, "candidate amount is missing").into(),
            )
        })?,
        currency: row.get(6)?,
        account_label: row.get(7)?,
    })
}

fn review_job_summary_from_row(row: &Row<'_>) -> rusqlite::Result<ReviewJobSummary> {
    let status = match row.get::<_, String>(1)?.as_str() {
        "blocked" => ReviewJobStatus::Blocked,
        "cancelled" => ReviewJobStatus::Cancelled,
        "failed" => ReviewJobStatus::Failed,
        "queued" => ReviewJobStatus::Queued,
        "running" => ReviewJobStatus::Running,
        "succeeded" => ReviewJobStatus::Succeeded,
        _ => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                io::Error::new(io::ErrorKind::InvalidData, "invalid review job status").into(),
            ));
        }
    };
    let result_json: Option<String> = row.get(4)?;
    let outcomes = match result_json {
        Some(result_json) => {
            serde_json::from_str::<StoredReviewJobResult>(&result_json)
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .outcomes
        }
        None => Vec::new(),
    };
    Ok(ReviewJobSummary {
        job_id: row.get(0)?,
        status,
        created_at: row.get(2)?,
        finished_at: row.get(3)?,
        outcomes,
    })
}

fn open_review_record_for_relationship(
    transaction: &rusqlite::Transaction<'_>,
    review_item_id: &str,
    expected_record_version: i64,
) -> StoreResult<Option<ReviewRelationshipRecord>> {
    let record = transaction
        .query_row(
            "SELECT external_records.id, external_records.event_type, \
                    external_records.account_balance_delta, external_records.currency \
             FROM review_items \
             JOIN external_records ON external_records.id = review_items.external_record_id \
             WHERE review_items.id = ?1 AND review_items.status = 'open' \
               AND external_records.version = ?2 \
               AND external_records.status IN ('staged', 'review')",
            params![review_item_id, expected_record_version],
            |row| {
                Ok(ReviewRelationshipRecord {
                    record_id: row.get(0)?,
                    event_type: row.get(1)?,
                    account_balance_delta: row.get(2)?,
                    currency: row.get(3)?,
                    review_item_id: Some(review_item_id.to_owned()),
                })
            },
        )
        .optional()?;
    Ok(record)
}

fn open_review_record_by_id(
    transaction: &rusqlite::Transaction<'_>,
    record_id: &str,
    expected_record_version: i64,
) -> StoreResult<Option<ReviewRelationshipRecord>> {
    let record = transaction
        .query_row(
            "SELECT external_records.id, external_records.event_type, \
                    external_records.account_balance_delta, external_records.currency, review_items.id \
             FROM external_records \
             LEFT JOIN review_items ON review_items.external_record_id = external_records.id \
                AND review_items.status = 'open' \
             WHERE external_records.id = ?1 AND external_records.version = ?2 \
               AND external_records.status IN ('staged', 'review') \
             ORDER BY review_items.id LIMIT 1",
            params![record_id, expected_record_version],
            |row| {
                Ok(ReviewRelationshipRecord {
                    record_id: row.get(0)?,
                    event_type: row.get(1)?,
                    account_balance_delta: row.get(2)?,
                    currency: row.get(3)?,
                    review_item_id: row.get(4)?,
                })
            },
        )
        .optional()?;
    Ok(record)
}

fn ordered_relationship_records<'a>(
    first: &'a ReviewRelationshipRecord,
    second: &'a ReviewRelationshipRecord,
) -> (&'a ReviewRelationshipRecord, &'a ReviewRelationshipRecord) {
    if first.record_id < second.record_id {
        (first, second)
    } else {
        (second, first)
    }
}

fn same_record_ids(record_ids: &[String], first: &str, second: &str) -> bool {
    if record_ids.len() != 2 {
        return false;
    }
    let mut expected = [first, second];
    expected.sort_unstable();
    let mut actual = [record_ids[0].as_str(), record_ids[1].as_str()];
    actual.sort_unstable();
    actual == expected
}

fn review_conflict(reason: &'static str) -> ReviewMutationOutcome {
    ReviewMutationOutcome {
        reason: Some(reason),
        record_version: None,
        review_item_id: None,
        status: ReviewMutationStatus::Conflict,
    }
}

fn valid_optional_date(value: Option<&str>) -> bool {
    value.is_none_or(valid_iso_date)
}

fn valid_optional_decimal(value: Option<&str>) -> bool {
    value.is_none_or(valid_exact_decimal)
}

fn valid_optional_non_negative_decimal(value: Option<&str>) -> bool {
    value.is_none_or(|value| !value.starts_with('-') && valid_exact_decimal(value))
}

fn valid_iso_date(value: &str) -> bool {
    parse_iso_date(value).is_some()
}

fn add_months_iso(value: &str, months: u32, preserve_month_end: bool) -> Option<String> {
    let (year, month, day) = parse_iso_date(value)?;
    let total_months = i64::from(year) * 12 + i64::from(month - 1) + i64::from(months);
    let target_year = i32::try_from(total_months.div_euclid(12)).ok()?;
    let target_month = u32::try_from(total_months.rem_euclid(12) + 1).ok()?;
    let target_month_days = days_in_month(target_year, target_month)?;
    let target_day = if preserve_month_end {
        target_month_days
    } else {
        day.min(target_month_days)
    };
    Some(format!(
        "{target_year:04}-{target_month:02}-{target_day:02}"
    ))
}

fn is_month_end_iso(value: &str) -> Option<bool> {
    let (year, month, day) = parse_iso_date(value)?;
    Some(day == days_in_month(year, month)?)
}

fn parse_iso_date(value: &str) -> Option<(i32, u32, u32)> {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = value[0..4].parse::<i32>().ok();
    let month = value[5..7].parse::<u32>().ok();
    let day = value[8..10].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return None;
    };
    let days_in_month = days_in_month(year, month)?;
    if !(1..=days_in_month).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn days_in_month(year: i32, month: u32) -> Option<u32> {
    Some(match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return None,
    })
}

fn valid_exact_decimal(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    let mut pieces = value.split('.');
    let Some(integer) = pieces.next() else {
        return false;
    };
    let fraction = pieces.next();
    if pieces.next().is_some()
        || integer.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || (integer.len() > 1 && integer.starts_with('0'))
    {
        return false;
    }
    fraction.is_none_or(|fraction| {
        !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn decimal_magnitude(value: &str) -> Option<String> {
    valid_exact_decimal(value).then(|| value.strip_prefix('-').unwrap_or(value).to_owned())
}

fn inverted_exact_decimal(value: &str) -> Option<String> {
    let magnitude = decimal_magnitude(value)?;
    if magnitude.bytes().all(|byte| byte == b'0' || byte == b'.') {
        return Some(magnitude);
    }
    Some(if value.starts_with('-') {
        magnitude
    } else {
        format!("-{magnitude}")
    })
}

fn matches_inverted_original_legs(
    original_legs: &[CoreReviewLeg],
    reversal_legs: &[CoreReviewLeg],
) -> bool {
    if original_legs.len() != 2 || reversal_legs.len() != 2 {
        return false;
    }
    let mut expected = Vec::with_capacity(2);
    for leg in original_legs {
        let Some(amount_value) = inverted_exact_decimal(&leg.amount_value) else {
            return false;
        };
        expected.push((
            leg.account_id.clone(),
            leg.instrument_id.clone(),
            leg.currency.clone(),
            amount_value,
        ));
    }
    let mut actual = reversal_legs
        .iter()
        .map(|leg| {
            (
                leg.account_id.clone(),
                leg.instrument_id.clone(),
                leg.currency.clone(),
                leg.amount_value.clone(),
            )
        })
        .collect::<Vec<_>>();
    expected.sort_unstable();
    actual.sort_unstable();
    expected == actual
}

fn valid_core_review_event(event: &CorePreparedReviewEvent) -> bool {
    event.event_class == "posting"
        && is_review_event_type(&event.event_type)
        && valid_iso_date(&event.event_date)
        && !event.spending
        && event.source_record_ids.len() == 2
        && event.source_record_ids.iter().all(|id| !id.is_empty())
        && event.source_record_ids[0] != event.source_record_ids[1]
        && event.legs.len() == 2
        && event.legs.iter().all(|leg| {
            !leg.account_id.is_empty()
                && !leg.instrument_id.is_empty()
                && !leg.currency.is_empty()
                && valid_exact_decimal(&leg.amount_value)
        })
}

fn valid_core_review_reversal(event: &CorePreparedReversalEvent) -> bool {
    matches!(
        event.event_type.as_str(),
        "same_currency_transfer_reversal" | "credit_card_repayment_reversal"
    ) && event.event_class == "posting"
        && valid_iso_date(&event.event_date)
        && !event.spending
        && event.legs.len() == 2
        && event.legs.iter().all(|leg| {
            !leg.account_id.is_empty()
                && !leg.instrument_id.is_empty()
                && !leg.currency.is_empty()
                && valid_exact_decimal(&leg.amount_value)
        })
}

fn is_review_event_type(event_type: &str) -> bool {
    matches!(
        event_type,
        "same_currency_transfer" | "credit_card_repayment"
    )
}

fn new_database_id(prefix: &str) -> String {
    let mut random = [0_u8; 16];
    OsRng.fill_bytes(&mut random);
    format!("{prefix}-{}", hex_encode(&random))
}

fn event_matches_group(event: &CorePreparedReviewEvent, group: &CommitReviewGroup) -> bool {
    if !same_record_ids(
        &event.source_record_ids,
        &group.records[0].id,
        &group.records[1].id,
    ) {
        return false;
    }
    group.records.iter().all(|record| {
        event.legs.iter().any(|leg| {
            leg.account_id == record.account_id
                && leg.instrument_id == record.instrument_id
                && leg.currency == record.currency
                && leg.amount_value == record.account_balance_delta
        })
    })
}

fn review_commit_key(event_type: &str, records: &[(String, String, i64, String)]) -> String {
    let mut proposal_versions = records
        .iter()
        .map(|(_, stable_record_key, version, _)| format!("{stable_record_key}:{version}"))
        .collect::<Vec<_>>();
    proposal_versions.sort();
    let digest = Sha256::digest(
        format!(
            "{REVIEW_POLICY_VERSION}:{event_type}:{}",
            proposal_versions.join("|")
        )
        .as_bytes(),
    );
    format!("review-{}", hex_encode(&digest))
}

fn needs_attention(document_id: &str, reason: &'static str) -> SourceDocumentRoutingOutcome {
    SourceDocumentRoutingOutcome::needs_attention(document_id, reason)
}

fn open_encrypted_database(
    path: &Path,
    master_key: &[u8; KEY_LEN],
    flags: OpenFlags,
) -> StoreResult<Connection> {
    let connection = Connection::open_with_flags(path, flags)?;
    let database_key = derive_database_key(master_key)?;
    let raw_key = hex_encode_secret(database_key.as_ref());
    let pragma = Zeroizing::new(format!("PRAGMA key = \"x'{}'\";", raw_key.as_str()));
    connection.execute_batch(pragma.as_str())?;
    let cipher_version: String =
        connection.query_row("PRAGMA cipher_version", [], |row| row.get(0))?;
    if cipher_version.trim().is_empty() {
        return Err(io::Error::other("SQLCipher did not report a cipher version").into());
    }
    connection.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(connection)
}

fn derive_database_key(master_key: &[u8; KEY_LEN]) -> StoreResult<Zeroizing<[u8; KEY_LEN]>> {
    let hkdf = Hkdf::<Sha256>::new(None, master_key);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    hkdf.expand(DATABASE_KEY_CONTEXT, key.as_mut())
        .map_err(|_| io::Error::other("database key derivation failed"))?;
    Ok(key)
}

fn apply_migrations(connection: &mut Connection) -> StoreResult<()> {
    apply_migration_set(connection, MIGRATIONS)
}

fn apply_migration_set(connection: &mut Connection, migrations: &[Migration]) -> StoreResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations ( \
           version INTEGER PRIMARY KEY, \
           applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP \
         );",
    )?;
    for migration in migrations {
        let applied = connection
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = ?1",
                [migration.version],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if applied {
            continue;
        }
        if migration.foreign_keys_off {
            connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
        }
        let result: StoreResult<()> = (|| {
            let transaction = connection.transaction()?;
            transaction.execute_batch(migration.sql)?;
            let violation = transaction
                .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()?;
            if violation.is_some() {
                return Err(io::Error::other(format!(
                    "migration {} violates foreign keys",
                    migration.version
                ))
                .into());
            }
            transaction.execute(
                "INSERT INTO schema_migrations(version) VALUES (?1)",
                [migration.version],
            )?;
            transaction.commit()?;
            Ok(())
        })();
        if migration.foreign_keys_off {
            connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        }
        result?;
    }
    Ok(())
}

fn find_exact_document(
    connection: &Connection,
    file_sha256: &str,
) -> rusqlite::Result<Option<ExistingDocument>> {
    connection
        .query_row(
            "SELECT id, encrypted_locator, file_sha256, file_state \
             FROM source_documents WHERE file_sha256 = ?1",
            [file_sha256],
            |row| {
                Ok(ExistingDocument {
                    document_id: row.get(0)?,
                    encrypted_locator: row.get(1)?,
                    file_sha256: row.get(2)?,
                    file_state: row.get(3)?,
                })
            },
        )
        .optional()
}

fn find_document_by_id(
    connection: &Connection,
    document_id: &str,
) -> rusqlite::Result<Option<ExistingDocument>> {
    connection
        .query_row(
            "SELECT id, encrypted_locator, file_sha256, file_state \
             FROM source_documents WHERE id = ?1",
            [document_id],
            |row| {
                Ok(ExistingDocument {
                    document_id: row.get(0)?,
                    encrypted_locator: row.get(1)?,
                    file_sha256: row.get(2)?,
                    file_state: row.get(3)?,
                })
            },
        )
        .optional()
}

fn persist_source_deletion(
    connection: &mut Connection,
    document: &ExistingDocument,
    audit_id: &str,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, 'source_file_deletion_decided', \
                   'user', 'user_requested', ?3, 'source-file-deletion-v1')",
        params![audit_id, document.document_id, document.file_sha256],
    )?;
    let changed = transaction.execute(
        "UPDATE source_documents \
         SET encrypted_locator = NULL, file_state = 'deleted', \
             deleted_at = CURRENT_TIMESTAMP, deletion_audit_id = ?1 \
         WHERE id = ?2 AND file_state = 'available'",
        params![audit_id, document.document_id],
    )?;
    if changed != 1 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    transaction.execute(
        "INSERT INTO review_items(id, external_record_id, reason_code, status) \
         SELECT ?1 || ':' || external_records.id, external_records.id, \
                'source_file_deleted', 'open' \
         FROM external_records \
         WHERE source_document_id = ?2 AND status IN ('staged', 'review') \
           AND NOT EXISTS ( \
             SELECT 1 FROM review_items \
             WHERE external_record_id = external_records.id \
               AND reason_code = 'source_file_deleted' AND status = 'open' \
           )",
        params![audit_id, document.document_id],
    )?;
    transaction.execute(
        "UPDATE external_records SET status = 'review' \
         WHERE source_document_id = ?1 AND status = 'staged'",
        [&document.document_id],
    )?;
    transaction.commit()
}

fn persist_import(
    connection: &mut Connection,
    input: &SourceDocumentImport<'_>,
    stored: &StoredFile,
) -> rusqlite::Result<SourceDocumentImportOutcome> {
    let transaction = connection.transaction()?;
    let existing = find_exact_document(&transaction, &stored.file_sha256)?;
    let (document_id, status) = if let Some(existing) = existing {
        let status = if existing.file_state == "available" && !stored.created {
            SourceDocumentImportStatus::AlreadyPresent
        } else {
            transaction.execute(
                "UPDATE source_documents \
                 SET encrypted_locator = ?1, file_state = 'available', \
                     deleted_at = NULL, deletion_audit_id = NULL \
                 WHERE id = ?2",
                params![stored.encrypted_locator, existing.document_id],
            )?;
            SourceDocumentImportStatus::Restored
        };
        (existing.document_id, status)
    } else {
        let byte_size = i64::try_from(stored.byte_size)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        transaction.execute(
            "INSERT INTO source_documents( \
               id, file_sha256, original_filename, mime_type, byte_size, \
               encrypted_locator, file_state \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'available')",
            params![
                input.document_id,
                stored.file_sha256,
                input.original_filename,
                input.mime_type,
                byte_size,
                stored.encrypted_locator,
            ],
        )?;
        (
            input.document_id.to_owned(),
            SourceDocumentImportStatus::Imported,
        )
    };
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            input.audit_id,
            document_id,
            import_audit_action(status),
            input.audit_actor,
            input.audit_reason,
            stored.file_sha256,
            input.audit_policy_version,
        ],
    )?;
    transaction.commit()?;
    Ok(SourceDocumentImportOutcome {
        document_id,
        status,
    })
}

fn mark_missing(
    connection: &mut Connection,
    document: &ExistingDocument,
    file_sha256: &str,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE source_documents \
         SET file_state = 'missing', deleted_at = NULL, deletion_audit_id = NULL \
         WHERE id = ?1",
        [&document.document_id],
    )?;
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'source_document', ?2, 'source_file_missing', \
                   'system', 'storage_verification_failed', ?3, 'vault-storage-v1')",
        params![new_audit_id(), document.document_id, file_sha256],
    )?;
    transaction.commit()
}

fn import_audit_action(status: SourceDocumentImportStatus) -> &'static str {
    match status {
        SourceDocumentImportStatus::Imported => "manual_import_imported",
        SourceDocumentImportStatus::AlreadyPresent => "manual_import_already_present",
        SourceDocumentImportStatus::Restored => "manual_import_restored",
        SourceDocumentImportStatus::RestoreConfirmationRequired => {
            "manual_import_restore_confirmation_required"
        }
    }
}

fn new_audit_id() -> String {
    let mut random = [0_u8; 16];
    OsRng.fill_bytes(&mut random);
    format!("audit-{}", hex_encode(&random))
}

fn validate_import(input: &SourceDocumentImport<'_>) -> StoreResult<()> {
    for (name, value) in [
        ("audit_actor", input.audit_actor),
        ("audit_id", input.audit_id),
        ("audit_policy_version", input.audit_policy_version),
        ("audit_reason", input.audit_reason),
        ("document_id", input.document_id),
        ("mime_type", input.mime_type),
        ("original_filename", input.original_filename),
    ] {
        if value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must not be empty"),
            )
            .into());
        }
    }
    Ok(())
}

fn validate_captured_container(mime_type: &str, plaintext: &[u8]) -> StoreResult<()> {
    match mime_type {
        "application/pdf" if !plaintext.starts_with(b"%PDF-") => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "captured PDF does not have a PDF header",
        )
        .into()),
        "text/csv" => {
            std::str::from_utf8(plaintext).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "captured CSV is not UTF-8")
            })?;
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .flexible(true)
                .from_reader(plaintext);
            for record in reader.records() {
                record.map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "captured CSV is malformed")
                })?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_classification(input: &TrustedDocumentClassification<'_>) -> StoreResult<()> {
    for (name, value) in [
        ("audit_id", input.audit_id),
        ("document_id", input.document_id),
        ("provider_key", input.provider_key),
        ("semantic_document_key", input.semantic_document_key),
    ] {
        if value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must not be empty"),
            )
            .into());
        }
    }
    if input.accounts.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "classification must contain at least one account",
        )
        .into());
    }
    if !valid_optional_date(input.statement_period_from)
        || !valid_optional_date(input.statement_period_to)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "statement period must contain valid ISO dates",
        )
        .into());
    }
    for account in input.accounts {
        for (name, value) in [
            ("account_id", account.account_id),
            ("account_type", account.account_type),
            ("display_name", account.display_name),
        ] {
            if value.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{name} must not be empty"),
                )
                .into());
            }
        }
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    hex
}

fn hex_encode_secret(bytes: &[u8]) -> Zeroizing<String> {
    let mut hex = Zeroizing::new(String::with_capacity(bytes.len() * 2));
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut *hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; KEY_LEN] = [0x91; KEY_LEN];

    fn open_store(root: &Path) -> ManualImportStore {
        let store =
            ManualImportStore::open(root, Zeroizing::new(KEY)).expect("open encrypted Vault");
        store
            .connection
            .execute(
                "INSERT OR IGNORE INTO money_sources( \
                   id, provider_key, display_name, source_type \
                 ) VALUES ('source-dbs', 'dbs', 'DBS', 'bank')",
                [],
            )
            .expect("seed money source");
        store
    }

    #[test]
    fn lists_only_safe_money_source_display_fields_in_stable_order() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let store = open_store(root.path());
        store
            .seed_money_source("source-alpha", "alpha", "Alpha Bank", "bank")
            .expect("seed second source");

        assert_eq!(
            store.list_money_sources().expect("list Money Sources"),
            vec![
                MoneySourceView {
                    display_name: "Alpha Bank".to_owned(),
                    money_source_id: "source-alpha".to_owned(),
                    source_type: "bank".to_owned(),
                },
                MoneySourceView {
                    display_name: "DBS".to_owned(),
                    money_source_id: "source-dbs".to_owned(),
                    source_type: "bank".to_owned(),
                },
            ]
        );
    }

    fn seed_candidate_account(
        store: &ManualImportStore,
        account_id: &str,
        money_source_id: &str,
        display_name: &str,
        account_type: &str,
        masked_identifier: Option<&str>,
        currency: Option<&str>,
    ) {
        store
            .connection
            .execute(
                "INSERT INTO accounts( \
                   id, money_source_id, provider_key, provider_account_id, account_type, \
                   display_name, masked_identifier, currency, status, raw_identity_json \
                 ) VALUES (?1, ?2, 'synthetic', ?3, ?4, ?5, ?6, ?7, 'candidate', ?8)",
                params![
                    account_id,
                    money_source_id,
                    format!("private-{account_id}"),
                    account_type,
                    display_name,
                    masked_identifier,
                    currency,
                    format!(r#"{{"providerAccountId":"private-{account_id}"}}"#),
                ],
            )
            .expect("seed candidate account");
    }

    #[test]
    fn lists_pending_account_confirmations_without_private_identity_fields() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let store = open_store(root.path());
        store
            .seed_money_source("source-alpha", "alpha", "Alpha Bank", "bank")
            .expect("seed source");
        seed_candidate_account(
            &store,
            "candidate-dbs",
            "source-dbs",
            "Everyday",
            "deposit_account",
            Some("••001"),
            Some("SGD"),
        );
        seed_candidate_account(
            &store,
            "candidate-alpha",
            "source-alpha",
            "Savings",
            "deposit_account",
            None,
            Some("USD"),
        );
        seed_candidate_account(
            &store,
            "confirmed-dbs",
            "source-dbs",
            "Confirmed",
            "deposit_account",
            None,
            Some("SGD"),
        );
        store
            .connection
            .execute(
                "UPDATE accounts SET status = 'confirmed' WHERE id = 'confirmed-dbs'",
                [],
            )
            .expect("confirm fixture account");

        let prompts = store
            .list_account_confirmation_prompts()
            .expect("list pending confirmations");

        assert_eq!(
            prompts,
            vec![
                AccountConfirmationPrompt {
                    candidate_accounts: vec![AccountConfirmationCandidate {
                        account_id: "candidate-alpha".to_owned(),
                        account_type: "deposit_account".to_owned(),
                        currency: Some("USD".to_owned()),
                        display_name: "Savings".to_owned(),
                        masked_identifier: None,
                    }],
                    display_name: "Alpha Bank".to_owned(),
                    money_source_id: "source-alpha".to_owned(),
                },
                AccountConfirmationPrompt {
                    candidate_accounts: vec![AccountConfirmationCandidate {
                        account_id: "candidate-dbs".to_owned(),
                        account_type: "deposit_account".to_owned(),
                        currency: Some("SGD".to_owned()),
                        display_name: "Everyday".to_owned(),
                        masked_identifier: Some("••001".to_owned()),
                    }],
                    display_name: "DBS".to_owned(),
                    money_source_id: "source-dbs".to_owned(),
                },
            ]
        );
        let serialized = serde_json::to_string(&prompts).expect("serialize safe prompts");
        assert!(!serialized.contains("providerAccountId"));
        assert!(!serialized.contains("providerKey"));
        assert!(!serialized.contains("rawIdentityJson"));
        assert!(!serialized.contains("private-candidate-dbs"));
    }

    #[test]
    fn confirms_the_exact_candidate_set_once() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_candidate_account(
            &store,
            "candidate-one",
            "source-dbs",
            "Everyday",
            "deposit_account",
            Some("••001"),
            Some("SGD"),
        );
        seed_candidate_account(
            &store,
            "candidate-two",
            "source-dbs",
            "Savings",
            "deposit_account",
            Some("••002"),
            Some("SGD"),
        );
        let expected = vec!["candidate-two".to_owned(), "candidate-one".to_owned()];

        assert_eq!(
            store
                .confirm_candidate_accounts("source-dbs", &expected, "audit-confirm-accounts")
                .expect("confirm exact candidate set"),
            AccountConfirmationOutcome {
                status: AccountConfirmationStatus::Confirmed,
            }
        );
        let statuses = store
            .connection
            .prepare("SELECT status FROM accounts WHERE id IN (?1, ?2) ORDER BY id")
            .expect("prepare account status query")
            .query_map(["candidate-one", "candidate-two"], |row| {
                row.get::<_, String>(0)
            })
            .expect("query account statuses")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect account statuses");
        assert_eq!(statuses, vec!["confirmed", "confirmed"]);
        let audit_count: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE entity_id = 'source-dbs' AND action = 'candidate_accounts_confirmed'",
                [],
                |row| row.get(0),
            )
            .expect("count confirmation audits");
        assert_eq!(audit_count, 1);

        assert_eq!(
            store
                .confirm_candidate_accounts("source-dbs", &expected, "audit-repeat")
                .expect("repeat confirmation"),
            AccountConfirmationOutcome {
                status: AccountConfirmationStatus::AlreadyConfirmed,
            }
        );
        let repeated_audit_count: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE entity_id = 'source-dbs' AND action = 'candidate_accounts_confirmed'",
                [],
                |row| row.get(0),
            )
            .expect("count repeated confirmation audits");
        assert_eq!(repeated_audit_count, 1);
    }

    #[test]
    fn rejects_a_stale_candidate_confirmation_without_writing() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_candidate_account(
            &store,
            "candidate-one",
            "source-dbs",
            "Everyday",
            "deposit_account",
            Some("••001"),
            Some("SGD"),
        );
        seed_candidate_account(
            &store,
            "candidate-two",
            "source-dbs",
            "Savings",
            "deposit_account",
            Some("••002"),
            Some("SGD"),
        );
        let expected = store
            .list_account_confirmation_prompts()
            .expect("list confirmation prompt")[0]
            .candidate_accounts
            .iter()
            .map(|candidate| candidate.account_id.clone())
            .collect::<Vec<_>>();
        seed_candidate_account(
            &store,
            "candidate-new",
            "source-dbs",
            "New account",
            "deposit_account",
            Some("••003"),
            Some("SGD"),
        );

        assert_eq!(
            store
                .confirm_candidate_accounts("source-dbs", &expected, "audit-stale")
                .expect("reject stale confirmation"),
            AccountConfirmationOutcome {
                status: AccountConfirmationStatus::Conflict,
            }
        );
        let candidate_count: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM accounts \
                 WHERE money_source_id = 'source-dbs' AND status = 'candidate'",
                [],
                |row| row.get(0),
            )
            .expect("count unchanged candidates");
        let audit_count: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE entity_id = 'source-dbs' AND action = 'candidate_accounts_confirmed'",
                [],
                |row| row.get(0),
            )
            .expect("count confirmation audits");
        assert_eq!(candidate_count, 3);
        assert_eq!(audit_count, 0);
    }

    fn import<'a>(
        source_path: &'a Path,
        document_id: &'a str,
        audit_id: &'a str,
    ) -> SourceDocumentImport<'a> {
        SourceDocumentImport {
            audit_actor: "user",
            audit_id,
            audit_policy_version: "manual-import-v1",
            audit_reason: "manual_import",
            document_id,
            mime_type: "application/pdf",
            original_filename: "DBS-July-2026.pdf",
            source_path,
        }
    }

    #[test]
    fn rejects_invalid_captured_pdf_and_csv_before_persistence() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        for (document_id, mime_type, filename, bytes) in [
            (
                "invalid-captured-pdf",
                "application/pdf",
                "invalid.pdf",
                b"not a PDF" as &[u8],
            ),
            (
                "invalid-captured-csv-utf8",
                "text/csv",
                "invalid-utf8.csv",
                b"column\n\xff",
            ),
        ] {
            let source_path = root.path().join(filename);
            assert!(
                store
                    .register_captured_import(
                        &SourceDocumentImport {
                            audit_actor: "system",
                            audit_id: document_id,
                            audit_policy_version: "manual-import-v1",
                            audit_reason: "local_inbox_import",
                            document_id,
                            mime_type,
                            original_filename: filename,
                            source_path: &source_path,
                        },
                        Zeroizing::new(bytes.to_vec()),
                        None,
                    )
                    .is_err(),
                "{filename} must not be persisted"
            );
        }
        let documents: i64 = store
            .connection
            .query_row("SELECT count(*) FROM source_documents", [], |row| {
                row.get(0)
            })
            .expect("count source documents");
        assert_eq!(documents, 0);
        assert!(!root.path().join("files").exists());
    }

    #[test]
    fn accepts_captured_pdf_with_a_pdf_header_without_parsing_it() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        let source_path = root.path().join("protected.pdf");
        let outcome = store
            .register_captured_import(
                &SourceDocumentImport {
                    audit_actor: "system",
                    audit_id: "captured-protected-pdf",
                    audit_policy_version: "manual-import-v1",
                    audit_reason: "local_inbox_import",
                    document_id: "captured-protected-pdf",
                    mime_type: "application/pdf",
                    original_filename: "protected.pdf",
                    source_path: &source_path,
                },
                Zeroizing::new(b"%PDF-1.7\n/Encrypt".to_vec()),
                None,
            )
            .expect("capture protected PDF");
        assert_eq!(outcome.status, SourceDocumentImportStatus::Imported);
    }

    #[test]
    fn source_document_pipeline_queues_reconciliation_only_after_parse_succeeds() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF synthetic statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .register_import(
                &import(&source_path, "document-local-inbox", "audit-import"),
                None,
            )
            .expect("import source");

        store
            .enqueue_source_document_pipeline("document-local-inbox")
            .expect("enqueue pipeline");
        let initial_jobs = store
            .connection
            .prepare(
                "SELECT job_type, status FROM jobs \
                 WHERE related_source_document_id = ?1 ORDER BY job_type",
            )
            .expect("prepare job query")
            .query_map(["document-local-inbox"], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query jobs")
            .collect::<Result<Vec<_>, _>>()
            .expect("read jobs");
        assert_eq!(
            initial_jobs,
            vec![
                ("parse_document".to_owned(), "queued".to_owned()),
                ("source_document_ingest".to_owned(), "succeeded".to_owned()),
            ]
        );

        assert!(
            store
                .start_parse_document("document-local-inbox")
                .expect("start parse")
        );
        store
            .finish_parse_document(
                "document-local-inbox",
                &SourceDocumentRoutingOutcome {
                    account_ids: vec!["account-dbs".to_owned()],
                    document_id: "document-local-inbox".to_owned(),
                    money_source_id: Some("source-dbs".to_owned()),
                    reason: None,
                    status: SourceDocumentRoutingStatus::Routed,
                },
            )
            .expect("finish parse");
        store
            .finish_parse_document(
                "document-local-inbox",
                &SourceDocumentRoutingOutcome {
                    account_ids: vec!["account-dbs".to_owned()],
                    document_id: "document-local-inbox".to_owned(),
                    money_source_id: Some("source-dbs".to_owned()),
                    reason: None,
                    status: SourceDocumentRoutingStatus::Routed,
                },
            )
            .expect("repeat completion");
        let final_jobs = store
            .connection
            .prepare(
                "SELECT job_type, status FROM jobs \
                 WHERE related_source_document_id = ?1 ORDER BY job_type",
            )
            .expect("prepare job query")
            .query_map(["document-local-inbox"], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query jobs")
            .collect::<Result<Vec<_>, _>>()
            .expect("read jobs");
        assert_eq!(
            final_jobs,
            vec![
                ("parse_document".to_owned(), "succeeded".to_owned()),
                ("reconcile_document".to_owned(), "queued".to_owned()),
                ("source_document_ingest".to_owned(), "succeeded".to_owned()),
            ]
        );
    }

    #[test]
    fn explicit_reenqueue_retries_only_failed_parse_jobs_with_attempts_left() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF transient sidecar failure").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .register_import(&import(&source_path, "document-retry", "audit-retry"), None)
            .expect("import source");
        store
            .enqueue_source_document_pipeline("document-retry")
            .expect("enqueue pipeline");
        assert!(
            store
                .start_parse_document("document-retry")
                .expect("start first parse")
        );
        store
            .fail_parse_document("document-retry", "normalizer_failed")
            .expect("fail transient parse");
        store
            .connection
            .execute(
                "UPDATE jobs \
                 SET error_json = '{\"code\":\"normalizer_failed\"}', result_json = '{}', \
                     lease_owner = 'failed-worker', lease_until = CURRENT_TIMESTAMP \
                 WHERE related_source_document_id = ?1 AND job_type = ?2",
                params!["document-retry", PARSE_DOCUMENT_JOB_TYPE],
            )
            .expect("seed terminal parse fields");

        store
            .enqueue_source_document_pipeline("document-retry")
            .expect("explicit re-enqueue retries transient failure");
        let retry_state: (String, i64, bool) = store
            .connection
            .query_row(
                "SELECT status, attempts, \
                        result_json IS NULL AND error_json IS NULL AND blocked_reason IS NULL \
                        AND lease_owner IS NULL AND lease_until IS NULL AND finished_at IS NULL \
                 FROM jobs WHERE related_source_document_id = ?1 AND job_type = ?2",
                params!["document-retry", PARSE_DOCUMENT_JOB_TYPE],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read re-queued parse");
        assert_eq!(retry_state, ("queued".to_owned(), 1, true));
        assert!(
            store
                .start_parse_document("document-retry")
                .expect("retry parse")
        );
        let retry_attempts: i64 = store
            .connection
            .query_row(
                "SELECT attempts FROM jobs \
                 WHERE related_source_document_id = ?1 AND job_type = ?2",
                params!["document-retry", PARSE_DOCUMENT_JOB_TYPE],
                |row| row.get(0),
            )
            .expect("read retry attempts");
        assert_eq!(retry_attempts, 2);

        for (status, attempts) in [("blocked", 2), ("succeeded", 2), ("failed", 3)] {
            store
                .connection
                .execute(
                    "UPDATE jobs SET status = ?1, attempts = ?2 \
                     WHERE related_source_document_id = ?3 AND job_type = ?4",
                    params![status, attempts, "document-retry", PARSE_DOCUMENT_JOB_TYPE],
                )
                .expect("seed ineligible parse state");
            store
                .enqueue_source_document_pipeline("document-retry")
                .expect("explicit re-enqueue leaves ineligible state alone");
            let actual: (String, i64) = store
                .connection
                .query_row(
                    "SELECT status, attempts FROM jobs \
                     WHERE related_source_document_id = ?1 AND job_type = ?2",
                    params!["document-retry", PARSE_DOCUMENT_JOB_TYPE],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("read ineligible parse state");
            assert_eq!(actual, (status.to_owned(), attempts));
        }
    }

    fn seed_coverage_statement(
        store: &mut ManualImportStore,
        document_id: &str,
        period_from: &str,
        period_to: &str,
    ) {
        store
            .connection
            .execute(
                "INSERT INTO source_documents( \
                   id, money_source_id, file_sha256, semantic_document_key, original_filename, \
                   mime_type, byte_size, encrypted_locator, file_state, document_type, \
                   statement_period_from, statement_period_to \
                 ) VALUES (?1, 'source-dbs', ?2, ?3, ?4, 'application/pdf', 1, ?5, \
                           'available', 'account_statement', ?6, ?7)",
                params![
                    document_id,
                    format!("{document_id:0<64}"),
                    format!("dbs:checking:{period_from}"),
                    format!("{document_id}.pdf"),
                    format!("files/{document_id}.ccenv"),
                    period_from,
                    period_to,
                ],
            )
            .expect("seed coverage document");
        store
            .connection
            .execute(
                "INSERT INTO source_document_accounts(source_document_id, account_id) \
                 VALUES (?1, 'account-dbs')",
                [document_id],
            )
            .expect("link coverage account");
    }

    #[test]
    fn derives_monthly_coverage_gaps_and_idempotent_user_decisions() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        store
            .connection
            .execute(
                "INSERT INTO accounts( \
                   id, money_source_id, provider_key, provider_account_id, account_type, \
                   display_name, currency, status \
                 ) VALUES ('account-dbs', 'source-dbs', 'dbs', 'checking-001', \
                           'deposit_account', 'DBS checking', 'SGD', 'confirmed')",
                [],
            )
            .expect("seed coverage account");
        seed_coverage_statement(&mut store, "document-january", "2026-01-01", "2026-01-31");
        seed_coverage_statement(&mut store, "document-march", "2026-03-01", "2026-03-31");
        let policy = [StatementCoveragePolicy {
            cadence_months: 1,
            document_type: "account_statement",
            grace_days: 7,
            provider_key: "dbs",
        }];
        let prompts = store
            .list_statement_coverage_prompts(&policy, "2026-05-10")
            .expect("derive coverage prompts");
        assert_eq!(
            prompts,
            vec![
                StatementCoveragePrompt {
                    account_id: "account-dbs".to_owned(),
                    document_type: "account_statement".to_owned(),
                    money_source_id: "source-dbs".to_owned(),
                    statement_period_from: "2026-02-01".to_owned(),
                    statement_period_to: "2026-02-28".to_owned(),
                    status: StatementCoveragePromptStatus::ConfirmedMissing,
                },
                StatementCoveragePrompt {
                    account_id: "account-dbs".to_owned(),
                    document_type: "account_statement".to_owned(),
                    money_source_id: "source-dbs".to_owned(),
                    statement_period_from: "2026-04-01".to_owned(),
                    statement_period_to: "2026-04-30".to_owned(),
                    status: StatementCoveragePromptStatus::LikelyMissing,
                },
            ]
        );

        let not_expected = StatementCoverageDecisionInput {
            account_id: "account-dbs",
            audit_id: "audit-coverage-february",
            decision: StatementCoverageDecision::NotExpected,
            document_type: "account_statement",
            money_source_id: "source-dbs",
            remind_after: None,
            statement_period_from: "2026-02-01",
            statement_period_to: "2026-02-28",
        };
        store
            .record_statement_coverage_decision(&not_expected)
            .expect("record not expected");
        store
            .record_statement_coverage_decision(&not_expected)
            .expect("repeat not expected");
        let audits: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE action = 'statement_coverage_decided' AND entity_id LIKE '%:2026-02-01:2026-02-28'",
                [],
                |row| row.get(0),
            )
            .expect("count coverage audits");
        assert_eq!(audits, 1);

        let remind_later = StatementCoverageDecisionInput {
            account_id: "account-dbs",
            audit_id: "audit-coverage-april",
            decision: StatementCoverageDecision::RemindLater,
            document_type: "account_statement",
            money_source_id: "source-dbs",
            remind_after: Some("2099-01-01"),
            statement_period_from: "2026-04-01",
            statement_period_to: "2026-04-30",
        };
        store
            .record_statement_coverage_decision(&remind_later)
            .expect("record reminder");
        assert!(
            store
                .list_statement_coverage_prompts(&policy, "2026-05-10")
                .expect("derive suppressed prompts")
                .is_empty()
        );
        assert_eq!(
            store
                .list_statement_coverage_prompts(&policy, "2100-01-01")
                .expect("derive reappeared prompts"),
            vec![StatementCoveragePrompt {
                account_id: "account-dbs".to_owned(),
                document_type: "account_statement".to_owned(),
                money_source_id: "source-dbs".to_owned(),
                statement_period_from: "2026-04-01".to_owned(),
                statement_period_to: "2026-04-30".to_owned(),
                status: StatementCoveragePromptStatus::LikelyMissing,
            }]
        );
    }

    #[test]
    fn keeps_month_end_when_multiple_monthly_periods_are_missing() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        store
            .connection
            .execute(
                "INSERT INTO accounts( \
                   id, money_source_id, provider_key, provider_account_id, account_type, \
                   display_name, currency, status \
                 ) VALUES ('account-dbs', 'source-dbs', 'dbs', 'checking-001', \
                           'deposit_account', 'DBS checking', 'SGD', 'confirmed')",
                [],
            )
            .expect("seed coverage account");
        seed_coverage_statement(&mut store, "document-january", "2026-01-01", "2026-01-31");
        seed_coverage_statement(&mut store, "document-april", "2026-04-01", "2026-04-30");
        let policy = [StatementCoveragePolicy {
            cadence_months: 1,
            document_type: "account_statement",
            grace_days: 7,
            provider_key: "dbs",
        }];

        let prompts = store
            .list_statement_coverage_prompts(&policy, "2026-04-01")
            .expect("derive missing periods");
        assert_eq!(
            prompts,
            vec![
                StatementCoveragePrompt {
                    account_id: "account-dbs".to_owned(),
                    document_type: "account_statement".to_owned(),
                    money_source_id: "source-dbs".to_owned(),
                    statement_period_from: "2026-02-01".to_owned(),
                    statement_period_to: "2026-02-28".to_owned(),
                    status: StatementCoveragePromptStatus::ConfirmedMissing,
                },
                StatementCoveragePrompt {
                    account_id: "account-dbs".to_owned(),
                    document_type: "account_statement".to_owned(),
                    money_source_id: "source-dbs".to_owned(),
                    statement_period_from: "2026-03-01".to_owned(),
                    statement_period_to: "2026-03-31".to_owned(),
                    status: StatementCoveragePromptStatus::ConfirmedMissing,
                },
            ]
        );
    }

    #[test]
    fn migrates_existing_document_relationships_to_pending_semantic_identity() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let database_path = root.path().join(DATABASE_FILE_NAME);
        let mut connection = open_encrypted_database(
            &database_path,
            &KEY,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .expect("open encrypted database");
        apply_migration_set(&mut connection, &MIGRATIONS[..2]).expect("apply old schema");
        connection
            .execute(
                "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
                 VALUES ('source-dbs', 'dbs', 'DBS', 'bank')",
                [],
            )
            .expect("seed source");
        connection
            .execute(
                "INSERT INTO source_documents( \
                   id, money_source_id, file_sha256, semantic_document_key, \
                   original_filename, mime_type, byte_size, encrypted_locator, file_state \
                 ) VALUES ( \
                   'document-existing', 'source-dbs', ?1, 'dbs:checking:2026-06', \
                   'DBS-June-2026.pdf', 'application/pdf', 2048, \
                   'files/document-existing.ccenv', 'available' \
                 )",
                ["e".repeat(64)],
            )
            .expect("seed document");
        connection
            .execute(
                "INSERT INTO parse_runs( \
                   id, source_document_id, normalization_profile_id, profile_json, status \
                 ) VALUES ( \
                   'parse-existing', 'document-existing', 'profile-v1', '{}', 'succeeded' \
                 )",
                [],
            )
            .expect("seed parse relationship");

        apply_migration_set(&mut connection, &MIGRATIONS[2..]).expect("apply pending identity");

        let not_null: i64 = connection
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('source_documents') \
                 WHERE name = 'semantic_document_key'",
                [],
                |row| row.get(0),
            )
            .expect("read semantic column");
        assert_eq!(not_null, 0);
        let semantic_document_key: String = connection
            .query_row(
                "SELECT semantic_document_key FROM source_documents \
                 WHERE id = 'document-existing'",
                [],
                |row| row.get(0),
            )
            .expect("preserve identity");
        assert_eq!(semantic_document_key, "dbs:checking:2026-06");
        let source_document_id: String = connection
            .query_row(
                "SELECT source_document_id FROM parse_runs WHERE id = 'parse-existing'",
                [],
                |row| row.get(0),
            )
            .expect("preserve parse relationship");
        assert_eq!(source_document_id, "document-existing");
        assert!(
            connection
                .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()
                .expect("check foreign keys")
                .is_none()
        );
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .expect("foreign keys enabled"),
            1
        );
        connection
            .execute(
                "INSERT INTO source_documents( \
                   id, money_source_id, file_sha256, \
                   original_filename, mime_type, byte_size, encrypted_locator, file_state \
                 ) VALUES ( \
                   'document-pending', 'source-dbs', ?1, \
                   'Pending.pdf', 'application/pdf', 1024, \
                   'files/document-pending.ccenv', 'available' \
                 )",
                ["p".repeat(64)],
            )
            .expect("insert pending identity");
        let pending_identity: Option<String> = connection
            .query_row(
                "SELECT semantic_document_key FROM source_documents \
                 WHERE id = 'document-pending'",
                [],
                |row| row.get(0),
            )
            .expect("read pending identity");
        assert_eq!(pending_identity, None);
        let source_not_null: i64 = connection
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('source_documents') \
                 WHERE name = 'money_source_id'",
                [],
                |row| row.get(0),
            )
            .expect("read source column");
        assert_eq!(source_not_null, 0);
        connection
            .execute(
                "INSERT INTO source_documents( \
                   id, file_sha256, original_filename, mime_type, byte_size, \
                   encrypted_locator, file_state \
                 ) VALUES ( \
                   'document-unassigned', ?1, 'Pending.csv', 'text/csv', 1024, \
                   'files/document-unassigned.ccenv', 'available' \
                 )",
                ["u".repeat(64)],
            )
            .expect("insert unassigned source");
        let pending_source: Option<String> = connection
            .query_row(
                "SELECT money_source_id FROM source_documents \
                 WHERE id = 'document-unassigned'",
                [],
                |row| row.get(0),
            )
            .expect("read pending source");
        assert_eq!(pending_source, None);

        let invalid = [Migration {
            version: 99,
            sql: "INSERT INTO parse_runs( \
                    id, source_document_id, normalization_profile_id, profile_json, status \
                  ) VALUES ( \
                    'parse-invalid', 'missing-document', 'profile-v1', '{}', 'failed' \
                  )",
            foreign_keys_off: true,
        }];
        assert!(apply_migration_set(&mut connection, &invalid).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .expect("foreign keys restored after failure"),
            1
        );
        let invalid_rows: i64 = connection
            .query_row(
                "SELECT count(*) FROM parse_runs WHERE id = 'parse-invalid'",
                [],
                |row| row.get(0),
            )
            .expect("invalid migration rolled back");
        assert_eq!(invalid_rows, 0);
    }

    #[test]
    fn routes_trusted_classification_to_one_source_and_reuses_the_account() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.csv");
        fs::write(&source_path, b"synthetic statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .register_import(
                &SourceDocumentImport {
                    audit_actor: "user",
                    audit_id: "audit-import",
                    audit_policy_version: "manual-import-v1",
                    audit_reason: "manual_import",
                    document_id: "document-unassigned",
                    mime_type: "text/csv",
                    original_filename: "statement.csv",
                    source_path: &source_path,
                },
                None,
            )
            .expect("capture unassigned source");
        let accounts = [TrustedAccountCandidate {
            account_id: "account-candidate",
            account_type: "deposit_account",
            currency: Some("SGD"),
            display_name: "Synthetic checking",
            masked_identifier: Some("••001"),
            provider_account_id: Some("checking-001"),
        }];
        let routed = store
            .apply_trusted_classification(&TrustedDocumentClassification {
                accounts: &accounts,
                audit_id: "audit-classify",
                document_id: "document-unassigned",
                document_type: Some("account_statement"),
                provider_key: "dbs",
                semantic_document_key: "dbs:checking:2026-07",
                statement_period_from: Some("2026-07-01"),
                statement_period_to: Some("2026-07-31"),
            })
            .expect("route classification");
        assert_eq!(routed.status, SourceDocumentRoutingStatus::Routed);
        assert_eq!(routed.money_source_id.as_deref(), Some("source-dbs"));
        assert_eq!(routed.account_ids, vec!["account-candidate"]);
        assert!(
            store
                .list_unassigned_documents()
                .expect("list pending")
                .is_empty()
        );
        let documents = store.list_documents("source-dbs").expect("list routed");
        assert_eq!(documents.len(), 1);
        assert_eq!(
            documents[0].semantic_document_key.as_deref(),
            Some("dbs:checking:2026-07")
        );

        let repeated_accounts = [TrustedAccountCandidate {
            account_id: "account-ignored",
            account_type: "deposit_account",
            currency: Some("SGD"),
            display_name: "Synthetic checking",
            masked_identifier: Some("••001"),
            provider_account_id: Some("checking-001"),
        }];
        let repeated = store
            .apply_trusted_classification(&TrustedDocumentClassification {
                accounts: &repeated_accounts,
                audit_id: "audit-classify-repeat",
                document_id: "document-unassigned",
                document_type: Some("account_statement"),
                provider_key: "dbs",
                semantic_document_key: "dbs:checking:2026-07",
                statement_period_from: Some("2026-07-01"),
                statement_period_to: Some("2026-07-31"),
            })
            .expect("repeat classification");
        assert_eq!(repeated.account_ids, vec!["account-candidate"]);
    }

    #[test]
    fn leaves_ambiguous_source_or_account_classification_unassigned() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF synthetic statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .connection
            .execute(
                "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
                 VALUES ('source-dbs-second', 'dbs', 'DBS second', 'bank')",
                [],
            )
            .expect("seed ambiguous source");
        store
            .register_import(
                &SourceDocumentImport {
                    audit_actor: "user",
                    audit_id: "audit-import",
                    audit_policy_version: "manual-import-v1",
                    audit_reason: "manual_import",
                    document_id: "document-unassigned",
                    mime_type: "application/pdf",
                    original_filename: "statement.pdf",
                    source_path: &source_path,
                },
                None,
            )
            .expect("capture unassigned source");
        let accounts = [TrustedAccountCandidate {
            account_id: "account-candidate",
            account_type: "deposit_account",
            currency: Some("SGD"),
            display_name: "Synthetic checking",
            masked_identifier: None,
            provider_account_id: Some("checking-001"),
        }];
        let ambiguous = store
            .apply_trusted_classification(&TrustedDocumentClassification {
                accounts: &accounts,
                audit_id: "audit-classify",
                document_id: "document-unassigned",
                document_type: Some("account_statement"),
                provider_key: "dbs",
                semantic_document_key: "dbs:checking:2026-07",
                statement_period_from: Some("2026-07-01"),
                statement_period_to: Some("2026-07-31"),
            })
            .expect("classify ambiguous source");
        assert_eq!(
            ambiguous.status,
            SourceDocumentRoutingStatus::NeedsAttention
        );
        assert_eq!(ambiguous.reason, Some("money_source_ambiguous"));
        assert_eq!(
            store
                .list_unassigned_documents()
                .expect("list pending")
                .len(),
            1
        );
        let account_count: i64 = store
            .connection
            .query_row("SELECT count(*) FROM accounts", [], |row| row.get(0))
            .expect("count accounts");
        assert_eq!(account_count, 0);
    }

    #[test]
    fn imports_deduplicates_groups_and_restores_source_documents() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let first_path = root.path().join("first.pdf");
        fs::write(&first_path, b"%PDF synthetic first statement").expect("write first fixture");
        let mut store = open_store(root.path());

        let first = store
            .register_import(&import(&first_path, "document-first", "audit-first"), None)
            .expect("first import");
        assert_eq!(first.status, SourceDocumentImportStatus::Imported);
        let duplicate = store
            .register_import(
                &import(&first_path, "document-ignored", "audit-duplicate"),
                None,
            )
            .expect("duplicate import");
        assert_eq!(duplicate.status, SourceDocumentImportStatus::AlreadyPresent);

        let second_path = root.path().join("second.pdf");
        fs::write(&second_path, b"%PDF synthetic rescanned statement")
            .expect("write second fixture");
        let second = store
            .register_import(
                &import(&second_path, "document-second", "audit-second"),
                None,
            )
            .expect("second import");
        assert_eq!(second.status, SourceDocumentImportStatus::Imported);

        let first_sha256 = FileVault::prepare(&first_path)
            .expect("prepare first source")
            .file_sha256()
            .to_owned();
        let first_document = find_exact_document(&store.connection, &first_sha256)
            .expect("find document")
            .expect("existing document");
        let encrypted_path = root.path().join(
            first_document
                .encrypted_locator
                .as_deref()
                .expect("available locator"),
        );
        store
            .connection
            .execute(
                "INSERT INTO parse_runs( \
                   id, source_document_id, normalization_profile_id, profile_json, status \
                 ) VALUES ('parse-first', ?1, 'profile-v1', '{}', 'succeeded')",
                [&first_document.document_id],
            )
            .expect("seed retained parse relationship");
        store
            .connection
            .execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, stable_record_key, version, \
                   status, record_type, raw_json, validation_json \
                 ) VALUES \
                   ('record-staged', 'parse-first', ?1, 'record-staged', 1, \
                    'staged', 'transaction', '{}', '{}'), \
                   ('record-review', 'parse-first', ?1, 'record-review', 1, \
                    'review', 'transaction', '{}', '{}'), \
                   ('record-committed', 'parse-first', ?1, 'record-committed', 1, \
                    'committed', 'transaction', '{}', '{}')",
                [&first_document.document_id],
            )
            .expect("seed linked records");

        store
            .delete_source_document(&first_document.document_id, "audit-delete")
            .expect("delete encrypted source");
        assert!(!encrypted_path.exists());
        let deleted = find_document_by_id(&store.connection, &first_document.document_id)
            .expect("find deleted document")
            .expect("deleted document remains");
        assert_eq!(deleted.file_state, "deleted");
        assert!(deleted.encrypted_locator.is_none());
        let retained_parses: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM parse_runs WHERE source_document_id = ?1",
                [&first_document.document_id],
                |row| row.get(0),
            )
            .expect("count retained parse relationships");
        assert_eq!(retained_parses, 1);
        let record_states = store
            .connection
            .prepare(
                "SELECT id, status FROM external_records \
                 WHERE source_document_id = ?1 ORDER BY id",
            )
            .expect("prepare linked record query")
            .query_map([&first_document.document_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query linked records")
            .collect::<Result<Vec<_>, _>>()
            .expect("read linked records");
        assert_eq!(
            record_states,
            vec![
                ("record-committed".to_owned(), "committed".to_owned()),
                ("record-review".to_owned(), "review".to_owned()),
                ("record-staged".to_owned(), "review".to_owned()),
            ]
        );
        let deletion_review_items: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM review_items \
                 WHERE reason_code = 'source_file_deleted' AND status = 'open'",
                [],
                |row| row.get(0),
            )
            .expect("count source deletion review items");
        assert_eq!(deletion_review_items, 2);

        assert!(
            store
                .delete_source_document(&first_document.document_id, "audit-delete-retry")
                .is_err()
        );
        let deletion_audits: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE entity_id = ?1 AND action = 'source_file_deletion_decided'",
                [&first_document.document_id],
                |row| row.get(0),
            )
            .expect("count deletion decisions after retry");
        assert_eq!(deletion_audits, 1);

        let confirmation = store
            .register_import(
                &import(&first_path, "document-ignored-restored", "audit-restored"),
                None,
            )
            .expect("request restore confirmation");
        assert_eq!(confirmation.document_id, first_document.document_id);
        assert_eq!(
            confirmation.status,
            SourceDocumentImportStatus::RestoreConfirmationRequired
        );
        assert!(!encrypted_path.exists());

        fs::write(&first_path, b"%PDF changed after restore confirmation")
            .expect("change selected source");
        assert!(
            store
                .register_import(
                    &import(&first_path, "document-changed", "audit-changed"),
                    Some(&first_document.document_id),
                )
                .is_err()
        );
        assert_eq!(store.list_unassigned_documents().expect("list").len(), 2);
        fs::write(&first_path, b"%PDF synthetic first statement")
            .expect("restore selected source fixture");

        let restored = store
            .register_import(
                &import(&first_path, "document-ignored-restored", "audit-restored"),
                Some(&first_document.document_id),
            )
            .expect("restore exact source");
        assert_eq!(restored.document_id, first_document.document_id);
        assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
        assert_eq!(store.list_unassigned_documents().expect("list").len(), 2);
    }

    #[test]
    fn deletion_recovery_removes_a_blob_left_after_the_tombstone_commit() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF deletion recovery statement").expect("write fixture");
        let mut store = open_store(root.path());
        let imported = store
            .register_import(
                &import(&source_path, "document-delete", "audit-import"),
                None,
            )
            .expect("import source");
        let document = find_document_by_id(&store.connection, &imported.document_id)
            .expect("find document")
            .expect("imported document");
        let encrypted_path = root.path().join(
            document
                .encrypted_locator
                .as_deref()
                .expect("available locator"),
        );

        persist_source_deletion(&mut store.connection, &document, "audit-delete")
            .expect("commit deletion decision");
        assert!(
            encrypted_path.exists(),
            "simulate crash before blob removal"
        );
        drop(store);

        let reopened = open_store(root.path());
        assert!(!encrypted_path.exists());
        let tombstone = find_document_by_id(&reopened.connection, &imported.document_id)
            .expect("find tombstone")
            .expect("document row retained");
        assert_eq!(tombstone.file_state, "deleted");
        let deletion_audits: i64 = reopened
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE entity_id = ?1 AND action = 'source_file_deletion_decided'",
                [&imported.document_id],
                |row| row.get(0),
            )
            .expect("count deletion audit");
        assert_eq!(deletion_audits, 1);
    }

    #[test]
    fn missing_storage_is_not_recorded_as_a_user_deletion() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF missing before deletion").expect("write fixture");
        let mut store = open_store(root.path());
        let imported = store
            .register_import(
                &import(&source_path, "document-missing-delete", "audit-import"),
                None,
            )
            .expect("import source");
        let locator = store.list_unassigned_documents().expect("list")[0]
            .encrypted_locator
            .clone()
            .expect("available locator");
        store
            .files
            .remove(&locator)
            .expect("remove encrypted fixture");

        assert!(
            store
                .delete_source_document(&imported.document_id, "audit-delete")
                .is_err()
        );
        let document = find_document_by_id(&store.connection, &imported.document_id)
            .expect("find document")
            .expect("document row retained");
        assert_eq!(document.file_state, "missing");
        let deletion_audits: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log \
                 WHERE entity_id = ?1 AND action = 'source_file_deletion_decided'",
                [&imported.document_id],
                |row| row.get(0),
            )
            .expect("count deletion audit");
        assert_eq!(deletion_audits, 0);
    }

    #[test]
    fn removes_a_new_encrypted_file_when_the_database_transaction_fails() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF rollback statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .connection
            .execute(
                "INSERT INTO audit_log( \
                   id, entity_type, entity_id, action, actor, reason, policy_version \
                 ) VALUES ('audit-conflict', 'test', 'test', 'seed', 'test', 'test', 'test')",
                [],
            )
            .expect("seed audit conflict");

        assert!(
            store
                .register_import(
                    &import(&source_path, "document-rollback", "audit-conflict"),
                    None,
                )
                .is_err()
        );
        let rows: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM source_documents WHERE id = 'document-rollback'",
                [],
                |row| row.get(0),
            )
            .expect("count rolled-back documents");
        assert_eq!(rows, 0);
        let files = root.path().join("files");
        assert_eq!(fs::read_dir(files).expect("files directory").count(), 0);
    }

    #[test]
    fn marks_an_absent_registered_file_missing_and_restores_it_on_reimport() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF missing statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .register_import(
                &import(&source_path, "document-missing", "audit-import"),
                None,
            )
            .expect("import source");
        let locator = store.list_unassigned_documents().expect("list")[0]
            .encrypted_locator
            .clone()
            .expect("available locator");
        store
            .files
            .remove(&locator)
            .expect("remove file outside database");
        drop(store);

        let mut reopened = open_store(root.path());
        assert_eq!(
            reopened.list_unassigned_documents().expect("list")[0].file_state,
            "missing"
        );
        let restored = reopened
            .register_import(
                &import(&source_path, "document-ignored", "audit-restore"),
                None,
            )
            .expect("restore missing source");
        assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
    }

    #[test]
    fn marks_a_tampered_registered_file_missing_on_reopen() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let source_path = root.path().join("statement.pdf");
        fs::write(&source_path, b"%PDF tampered statement").expect("write fixture");
        let mut store = open_store(root.path());
        store
            .register_import(
                &import(&source_path, "document-tampered", "audit-import"),
                None,
            )
            .expect("import source");
        let locator = store.list_unassigned_documents().expect("list")[0]
            .encrypted_locator
            .clone()
            .expect("available locator");
        let encrypted_path = root.path().join(locator);
        let mut envelope = fs::read(&encrypted_path).expect("read encrypted fixture");
        *envelope.last_mut().expect("non-empty envelope") ^= 1;
        fs::write(encrypted_path, envelope).expect("tamper encrypted fixture");
        drop(store);

        let reopened = open_store(root.path());
        assert_eq!(
            reopened.list_unassigned_documents().expect("list")[0].file_state,
            "missing"
        );
    }

    #[test]
    fn restores_a_valid_envelope_found_at_the_wrong_document_locator() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let first_path = root.path().join("first.pdf");
        let second_path = root.path().join("second.pdf");
        fs::write(&first_path, b"%PDF first statement").expect("write first fixture");
        fs::write(&second_path, b"%PDF second statement").expect("write second fixture");
        let mut store = open_store(root.path());
        store
            .register_import(&import(&first_path, "document-first", "audit-first"), None)
            .expect("import first source");
        store
            .register_import(
                &import(&second_path, "document-second", "audit-second"),
                None,
            )
            .expect("import second source");

        let first_sha256 = FileVault::prepare(&first_path)
            .expect("prepare first source")
            .file_sha256()
            .to_owned();
        let second_sha256 = FileVault::prepare(&second_path)
            .expect("prepare second source")
            .file_sha256()
            .to_owned();
        let first = find_exact_document(&store.connection, &first_sha256)
            .expect("find first document")
            .expect("first document");
        let second = find_exact_document(&store.connection, &second_sha256)
            .expect("find second document")
            .expect("second document");
        let first_envelope = fs::read(
            root.path()
                .join(first.encrypted_locator.expect("first locator")),
        )
        .expect("read first envelope");
        fs::write(
            root.path()
                .join(second.encrypted_locator.expect("second locator")),
            first_envelope,
        )
        .expect("misplace valid envelope");
        drop(store);

        let mut store = open_store(root.path());
        let documents = store.list_unassigned_documents().expect("list");
        let second_document = documents
            .iter()
            .find(|document| document.document_id == "document-second")
            .expect("second document view");
        assert_eq!(second_document.file_state, "missing");

        let restored = store
            .register_import(
                &import(&second_path, "document-ignored", "audit-restored"),
                None,
            )
            .expect("restore second source");
        assert_eq!(restored.document_id, "document-second");
        assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
    }

    #[test]
    fn rejects_the_wrong_database_key_and_cleans_an_unregistered_envelope_on_open() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let orphan_path = root.path().join("orphan.pdf");
        fs::write(&orphan_path, b"unregistered evidence").expect("write orphan source");
        let store = open_store(root.path());
        let orphan = store
            .files
            .store(&KEY, &orphan_path)
            .expect("store orphaned envelope");
        let encrypted_path = root.path().join(&orphan.encrypted_locator);
        assert!(encrypted_path.exists());
        drop(store);

        let reopened = open_store(root.path());
        assert!(!encrypted_path.exists());
        drop(reopened);
        assert!(ManualImportStore::open(root.path(), Zeroizing::new([0x92; KEY_LEN])).is_err());
        let database_bytes =
            fs::read(root.path().join("finance.sqlite")).expect("read encrypted database");
        assert!(
            !database_bytes
                .windows("source-dbs".len())
                .any(|part| part == b"source-dbs")
        );
    }

    #[test]
    fn persists_only_statement_password_reference_state() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let store = open_store(root.path());

        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("initial statement password state"),
            None
        );

        store
            .begin_statement_password_save("source-dbs", "money-source:source-dbs")
            .expect("begin save");
        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("pending save state"),
            Some(StatementPasswordState {
                money_source_id: "source-dbs".to_owned(),
                secret_storage_key: "money-source:source-dbs".to_owned(),
                status: StatementPasswordStatus::PendingSave,
            })
        );
        assert_eq!(
            store
                .pending_statement_password_states()
                .expect("pending states")
                .len(),
            1
        );

        store
            .mark_statement_password_saved("source-dbs")
            .expect("finish save");
        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("saved state")
                .expect("saved reference")
                .status,
            StatementPasswordStatus::Saved
        );

        store
            .begin_statement_password_delete("source-dbs")
            .expect("begin delete");
        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("pending delete state")
                .expect("pending delete reference")
                .status,
            StatementPasswordStatus::PendingDelete
        );
        store
            .remove_statement_password_ref("source-dbs", "pending_delete")
            .expect("finish delete");
        assert_eq!(
            store
                .statement_password_state("source-dbs")
                .expect("cleared statement password state"),
            None
        );
    }

    fn seed_review_repayment(store: &mut ManualImportStore, card_first: bool) {
        store
            .seed_money_source("source-hsbc", "hsbc", "HSBC", "bank")
            .expect("seed HSBC source");
        store
            .connection
            .execute(
                "INSERT INTO accounts( \
                   id, money_source_id, provider_key, provider_account_id, account_type, \
                   display_name, currency, status \
                 ) VALUES \
                   ('account-hsbc-cash', 'source-hsbc', 'hsbc', 'cash-1', 'deposit_account', \
                    'HSBC Everyday', 'SGD', 'confirmed'), \
                   ('account-dbs-card', 'source-dbs', 'dbs', 'card-1', 'credit_card', \
                    'DBS Visa', 'SGD', 'confirmed')",
                [],
            )
            .expect("seed accounts");
        store
            .connection
            .execute(
                "INSERT INTO instruments(id, instrument_type, symbol, currency, display_name) \
                 VALUES ('instrument-sgd', 'fiat_currency', 'SGD', 'SGD', 'Singapore Dollar')",
                [],
            )
            .expect("seed instrument");
        let documents = if card_first {
            [
                (
                    "document-dbs-july",
                    "source-dbs",
                    "d".repeat(64),
                    "dbs:card:2026-07",
                ),
                (
                    "document-hsbc-june",
                    "source-hsbc",
                    "h".repeat(64),
                    "hsbc:cash:2026-06",
                ),
            ]
        } else {
            [
                (
                    "document-hsbc-june",
                    "source-hsbc",
                    "h".repeat(64),
                    "hsbc:cash:2026-06",
                ),
                (
                    "document-dbs-july",
                    "source-dbs",
                    "d".repeat(64),
                    "dbs:card:2026-07",
                ),
            ]
        };
        for (id, source_id, sha, semantic_key) in documents {
            store
                .connection
                .execute(
                    "INSERT INTO source_documents( \
                       id, money_source_id, file_sha256, semantic_document_key, \
                       original_filename, mime_type, byte_size, file_state \
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 'text/csv', 1, 'missing')",
                    params![id, source_id, sha, semantic_key, format!("{id}.csv")],
                )
                .expect("seed statement document");
            store
                .connection
                .execute(
                    "INSERT INTO parse_runs( \
                       id, source_document_id, normalization_profile_id, profile_json, status \
                     ) VALUES (?1, ?2, 'synthetic-review-v1', '{}', 'validated')",
                    params![format!("parse-{id}"), id],
                )
                .expect("seed parse run");
        }
        let records = if card_first {
            [
                (
                    "record-dbs-card",
                    "parse-document-dbs-july",
                    "document-dbs-july",
                    "account-dbs-card",
                    "dbs-card-payment",
                    "2026-07-01",
                ),
                (
                    "record-hsbc-cash",
                    "parse-document-hsbc-june",
                    "document-hsbc-june",
                    "account-hsbc-cash",
                    "hsbc-card-payment",
                    "2026-06-30",
                ),
            ]
        } else {
            [
                (
                    "record-hsbc-cash",
                    "parse-document-hsbc-june",
                    "document-hsbc-june",
                    "account-hsbc-cash",
                    "hsbc-card-payment",
                    "2026-06-30",
                ),
                (
                    "record-dbs-card",
                    "parse-document-dbs-july",
                    "document-dbs-july",
                    "account-dbs-card",
                    "dbs-card-payment",
                    "2026-07-01",
                ),
            ]
        };
        for (id, parse_run_id, document_id, account_id, stable_key, posted_on) in records {
            store
                .connection
                .execute(
                    "INSERT INTO external_records( \
                       id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                       status, record_type, event_type, posted_on, amount_value, currency, \
                       account_balance_delta, raw_json, validation_json \
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 1, 'review', 'transaction', \
                               'credit_card_repayment', ?6, '750.00', 'SGD', '-750.00', \
                               '{\"private\":\"must-not-leak\"}', '{\"internal\":true}')",
                    params![id, parse_run_id, document_id, account_id, stable_key, posted_on],
                )
                .expect("seed review record");
            store
                .connection
                .execute(
                    "INSERT INTO review_items(id, external_record_id, reason_code, status) \
                     VALUES (?1, ?2, 'possible_card_repayment', 'open')",
                    params![format!("review-{id}"), id],
                )
                .expect("seed review item");
        }
    }

    fn prepared_repayment() -> CorePreparedReviewEvent {
        CorePreparedReviewEvent {
            event_class: "posting".to_owned(),
            event_date: "2026-06-30".to_owned(),
            event_type: "credit_card_repayment".to_owned(),
            source_record_ids: vec!["record-dbs-card".to_owned(), "record-hsbc-cash".to_owned()],
            legs: vec![
                CoreReviewLeg {
                    account_id: "account-dbs-card".to_owned(),
                    instrument_id: "instrument-sgd".to_owned(),
                    currency: "SGD".to_owned(),
                    amount_value: "-750.00".to_owned(),
                },
                CoreReviewLeg {
                    account_id: "account-hsbc-cash".to_owned(),
                    instrument_id: "instrument-sgd".to_owned(),
                    currency: "SGD".to_owned(),
                    amount_value: "-750.00".to_owned(),
                },
            ],
            spending: false,
        }
    }

    #[test]
    fn limits_relationship_candidates_within_the_supported_window_before_capping_results() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_review_repayment(&mut store, false);

        for index in 0..101 {
            let id = format!("record-old-{index:03}");
            store
                .connection
                .execute(
                    "INSERT INTO external_records( \
                       id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                       status, record_type, event_type, posted_on, amount_value, currency, \
                       account_balance_delta, raw_json, validation_json \
                     ) VALUES (?1, 'parse-document-dbs-july', 'document-dbs-july', \
                               'account-dbs-card', ?2, 1, 'review', 'transaction', \
                               'credit_card_repayment', '2025-01-01', '750.00', 'SGD', '-750.00', \
                               '{}', '{}')",
                    params![id, format!("old-repayment-{index}")],
                )
                .expect("seed older relationship candidate");
        }

        let candidate_input = store
            .relationship_candidate_input("review-record-hsbc-cash", 1)
            .expect("load candidate input")
            .expect("current review record");
        assert_eq!(
            candidate_input
                .candidates
                .iter()
                .map(|candidate| candidate.id.as_str())
                .collect::<Vec<_>>(),
            vec!["record-dbs-card"]
        );
    }

    #[test]
    fn qualifies_relationship_candidates_before_capping_results() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_review_repayment(&mut store, false);

        for index in 0..101 {
            let id = format!("record-mismatch-{index:03}");
            store
                .connection
                .execute(
                    "INSERT INTO external_records( \
                       id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                       status, record_type, event_type, posted_on, amount_value, currency, \
                       account_balance_delta, raw_json, validation_json \
                     ) VALUES (?1, 'parse-document-dbs-july', 'document-dbs-july', \
                               'account-dbs-card', ?2, 1, 'review', 'transaction', \
                               'credit_card_repayment', '2026-06-30', '1.00', 'SGD', '-1.00', \
                               '{}', '{}')",
                    params![id, format!("mismatched-repayment-{index}")],
                )
                .expect("seed magnitude-mismatched candidate");
        }
        store
            .connection
            .execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                   status, record_type, event_type, posted_on, amount_value, currency, \
                   account_balance_delta, raw_json, validation_json \
                 ) VALUES ('record-dbs-card-second', 'parse-document-dbs-july', \
                           'document-dbs-july', 'account-dbs-card', 'dbs-card-repayment-second', \
                           1, 'review', 'transaction', 'credit_card_repayment', '2026-07-02', \
                           '750.00', 'SGD', '-750.0', '{}', '{}')",
                [],
            )
            .expect("seed second qualified candidate");

        let candidate_input = store
            .relationship_candidate_input("review-record-hsbc-cash", 1)
            .expect("load candidate input")
            .expect("current review record");
        assert_eq!(
            candidate_input
                .candidates
                .iter()
                .map(|candidate| candidate.id.as_str())
                .collect::<Vec<_>>(),
            vec!["record-dbs-card", "record-dbs-card-second"]
        );
    }

    #[test]
    fn advertises_undo_only_for_supported_review_event_types() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let store = open_store(root.path());
        for (id, event_type) in [
            ("event-repayment", "credit_card_repayment"),
            ("event-transfer", "same_currency_transfer"),
            ("event-purchase", "purchase"),
            ("event-other", "manual_adjustment"),
        ] {
            store
                .connection
                .execute(
                    "INSERT INTO ledger_events( \
                       id, event_type, event_class, event_date, status, commit_idempotency_key \
                     ) VALUES (?1, ?2, 'posting', '2026-07-01', 'committed', ?3)",
                    params![id, event_type, format!("activity-{id}")],
                )
                .expect("seed committed activity");
        }

        let activity = store.list_recent_activity().expect("list activity");
        assert!(
            activity
                .iter()
                .find(|item| item.event_id == "event-repayment")
                .expect("repayment activity")
                .can_undo
        );
        assert!(
            activity
                .iter()
                .find(|item| item.event_id == "event-transfer")
                .expect("transfer activity")
                .can_undo
        );
        assert!(
            !activity
                .iter()
                .find(|item| item.event_id == "event-purchase")
                .expect("purchase activity")
                .can_undo
        );
        assert!(
            !activity
                .iter()
                .find(|item| item.event_id == "event-other")
                .expect("other activity")
                .can_undo
        );
    }

    #[test]
    fn keeps_an_accepted_relationship_in_review_until_both_sides_are_selected() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_review_repayment(&mut store, false);
        store
            .accept_review_relationship(
                "review-record-hsbc-cash",
                1,
                "record-dbs-card",
                1,
                &prepared_repayment(),
            )
            .expect("accept exact repayment relationship");
        let job = store
            .enqueue_commit_review_batch(&["review-record-hsbc-cash".to_owned()])
            .expect("enqueue one selected side");
        let claimed = store
            .claim_review_batch(&job.job_id, "test-worker")
            .expect("claim batch")
            .expect("queued job");

        let (groups, outcomes) = store
            .prepare_commit_review_groups(&claimed)
            .expect("prepare selected review items");

        assert!(groups.is_empty());
        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            outcomes[0],
            ReviewBatchGroupOutcome {
                reason: Some("relationship_not_selected".to_owned()),
                record_ids: vec!["record-hsbc-cash".to_owned()],
                status: ReviewBatchGroupStatus::StillNeedsReview,
            }
        );
    }

    #[test]
    fn blocks_review_commit_until_relationship_accounts_are_confirmed() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_review_repayment(&mut store, false);
        store
            .connection
            .execute(
                "UPDATE accounts SET status = 'candidate' WHERE id = 'account-dbs-card'",
                [],
            )
            .expect("make card account a candidate");
        store
            .accept_review_relationship(
                "review-record-hsbc-cash",
                1,
                "record-dbs-card",
                1,
                &prepared_repayment(),
            )
            .expect("accept exact repayment relationship");
        let job = store
            .enqueue_commit_review_batch(&[
                "review-record-hsbc-cash".to_owned(),
                "review-record-dbs-card".to_owned(),
            ])
            .expect("enqueue selected relationship");
        let claimed = store
            .claim_review_batch(&job.job_id, "test-worker")
            .expect("claim batch")
            .expect("queued job");
        let (groups, outcomes) = store
            .prepare_commit_review_groups(&claimed)
            .expect("prepare selected relationship");
        assert!(outcomes.is_empty());
        assert_eq!(groups.len(), 1);

        let blocked = store
            .commit_prepared_review_group(&claimed, &groups[0], &prepared_repayment())
            .expect("block candidate account commit");

        assert_eq!(blocked.status, ReviewBatchGroupStatus::StillNeedsReview);
        assert_eq!(
            blocked.reason.as_deref(),
            Some("account_confirmation_required")
        );
        let event_count: i64 = store
            .connection
            .query_row("SELECT count(*) FROM ledger_events", [], |row| row.get(0))
            .expect("count ledger events");
        assert_eq!(event_count, 0);

        assert_eq!(
            store
                .confirm_candidate_accounts(
                    "source-dbs",
                    &["account-dbs-card".to_owned()],
                    "audit-confirm-card",
                )
                .expect("confirm candidate account"),
            AccountConfirmationOutcome {
                status: AccountConfirmationStatus::Confirmed,
            }
        );
        let committed = store
            .commit_prepared_review_group(&claimed, &groups[0], &prepared_repayment())
            .expect("commit after account confirmation");
        assert_eq!(committed.status, ReviewBatchGroupStatus::Committed);
        let committed_event_count: i64 = store
            .connection
            .query_row("SELECT count(*) FROM ledger_events", [], |row| row.get(0))
            .expect("count committed ledger events");
        assert_eq!(committed_event_count, 1);
    }

    #[test]
    fn persists_cross_month_repayment_review_commit_undo_and_restart_recovery() {
        for card_first in [false, true] {
            let root = tempfile::tempdir().expect("temporary Vault");
            let mut store = open_store(root.path());
            seed_review_repayment(&mut store, card_first);

            let candidate_input = store
                .relationship_candidate_input("review-record-hsbc-cash", 1)
                .expect("load candidate input")
                .expect("current review record");
            assert_eq!(candidate_input.primary_record_id, "record-hsbc-cash");
            assert_eq!(candidate_input.record.posted_on, "2026-06-30");
            assert_eq!(candidate_input.event_type, "credit_card_repayment");
            assert_eq!(
                candidate_input
                    .candidates
                    .iter()
                    .map(|candidate| candidate.id.as_str())
                    .collect::<Vec<_>>(),
                vec!["record-dbs-card"]
            );
            let accepted = store
                .accept_review_relationship(
                    "review-record-hsbc-cash",
                    1,
                    "record-dbs-card",
                    1,
                    &prepared_repayment(),
                )
                .expect("accept exact repayment relationship");
            assert_eq!(accepted.status, ReviewMutationStatus::RelationshipAccepted);
            let detail = store
                .review_item_detail("review-record-hsbc-cash")
                .expect("read review detail")
                .expect("detail remains available before commit");
            let serialized = serde_json::to_string(&detail).expect("serialize safe detail");
            assert!(!serialized.contains("rawJson"));
            assert!(!serialized.contains("validationJson"));
            assert!(!serialized.contains("private"));
            assert!(!serialized.contains("locator"));

            let job = store
                .enqueue_commit_review_batch(&[
                    "review-record-hsbc-cash".to_owned(),
                    "review-record-dbs-card".to_owned(),
                ])
                .expect("enqueue batch");
            let claimed = store
                .claim_review_batch(&job.job_id, "test-worker")
                .expect("claim batch")
                .expect("queued job claims once");
            let (groups, initial_outcomes) = store
                .prepare_commit_review_groups(&claimed)
                .expect("group accepted relationship");
            assert!(initial_outcomes.is_empty());
            assert_eq!(groups.len(), 1);
            let committed = store
                .commit_prepared_review_group(&claimed, &groups[0], &prepared_repayment())
                .expect("commit prepared repayment");
            assert_eq!(committed.status, ReviewBatchGroupStatus::Committed);
            store
                .connection
                .execute(
                    "UPDATE jobs SET lease_until = datetime('now', '-1 second') WHERE id = ?1",
                    [&job.job_id],
                )
                .expect("simulate crash after group commit");
            drop(store);

            let mut reopened = open_store(root.path());
            let recovered = reopened
                .review_job(&job.job_id)
                .expect("read recovered job")
                .expect("job");
            assert_eq!(recovered.status, ReviewJobStatus::Queued);
            assert_eq!(
                reopened
                    .queued_review_job_ids()
                    .expect("list recovered review jobs"),
                vec![job.job_id.clone()]
            );
            let retry = reopened
                .claim_review_batch(&job.job_id, "retry-worker")
                .expect("claim recovered job")
                .expect("recovered claim");
            let (retry_groups, outcomes) = reopened
                .prepare_commit_review_groups(&retry)
                .expect("rebuild recovered groups");
            assert!(retry_groups.is_empty());
            assert_eq!(outcomes.len(), 1);
            assert_eq!(outcomes[0].status, ReviewBatchGroupStatus::AlreadyCommitted);
            reopened
                .finish_review_batch(&retry, &outcomes)
                .expect("finish recovered job");
            let event_count: i64 = reopened
                .connection
                .query_row(
                    "SELECT count(*) FROM ledger_events WHERE event_type = 'credit_card_repayment'",
                    [],
                    |row| row.get(0),
                )
                .expect("count committed events");
            let audit_count: i64 = reopened
                .connection
                .query_row(
                    "SELECT count(*) FROM audit_log WHERE action = 'review_batch_committed'",
                    [],
                    |row| row.get(0),
                )
                .expect("count commit audit");
            assert_eq!(event_count, 1);
            assert_eq!(audit_count, 1);
            assert!(
                reopened
                    .list_recent_activity()
                    .expect("recent activity")
                    .iter()
                    .all(|item| !item.spending)
            );

            let original_id: String = reopened
                .connection
                .query_row(
                    "SELECT id FROM ledger_events WHERE event_type = 'credit_card_repayment'",
                    [],
                    |row| row.get(0),
                )
                .expect("committed original");
            let original = reopened
                .committed_review_event_for_reversal(&original_id)
                .expect("load original for undo")
                .expect("undo available");
            let reversal = CorePreparedReversalEvent {
                event_type: "credit_card_repayment_reversal".to_owned(),
                event_class: "posting".to_owned(),
                event_date: original.event_date.clone(),
                legs: original
                    .legs
                    .iter()
                    .map(|leg| CoreReviewLeg {
                        account_id: leg.account_id.clone(),
                        instrument_id: leg.instrument_id.clone(),
                        currency: leg.currency.clone(),
                        amount_value: "750.00".to_owned(),
                    })
                    .collect(),
                spending: false,
            };
            let mut wrong_date = reversal.clone();
            wrong_date.event_date = "2026-07-02".to_owned();
            assert_eq!(
                reopened
                    .persist_review_reversal(&original_id, &wrong_date)
                    .expect("reject wrong reversal date"),
                None
            );
            let mut wrong_leg = reversal.clone();
            wrong_leg.legs[0].amount_value = "751.00".to_owned();
            assert_eq!(
                reopened
                    .persist_review_reversal(&original_id, &wrong_leg)
                    .expect("reject wrong reversal leg"),
                None
            );
            assert_eq!(
                reopened
                    .persist_review_reversal(&original_id, &reversal)
                    .expect("persist undo")
                    .expect("undo outcome")
                    .status,
                UndoStatus::Undone
            );
            assert_eq!(
                reopened
                    .persist_review_reversal(&original_id, &reversal)
                    .expect("repeat undo")
                    .expect("idempotent undo outcome")
                    .status,
                UndoStatus::AlreadyUndone
            );
            assert_eq!(
                reopened
                    .connection
                    .query_row(
                        "SELECT event_date FROM ledger_events WHERE id = ?1",
                        [&original_id],
                        |row| row.get::<_, String>(0),
                    )
                    .expect("original stays immutable"),
                "2026-06-30"
            );
        }
    }

    #[test]
    fn rejects_a_stale_review_mutation_without_erasing_raw_or_audit_history() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_review_repayment(&mut store, false);

        let edited = store
            .edit_review_record("review-record-hsbc-cash", 1, Some("2026-06-29"), None, None)
            .expect("edit current review record");
        assert_eq!(edited.status, ReviewMutationStatus::Updated);
        assert_eq!(
            store
                .remove_review_record("review-record-hsbc-cash", 1)
                .expect("stale remove is safe"),
            review_conflict("stale_review_item")
        );
        assert_eq!(
            store
                .connection
                .query_row(
                    "SELECT raw_json FROM external_records WHERE id = 'record-hsbc-cash'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("original raw record remains"),
            "{\"private\":\"must-not-leak\"}"
        );
        let audit_count: i64 = store
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log WHERE action = 'review_record_edited'",
                [],
                |row| row.get(0),
            )
            .expect("one edit audit");
        assert_eq!(audit_count, 1);
    }

    #[test]
    fn rejects_negative_amount_value_edits_but_allows_signed_balance_delta() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_review_repayment(&mut store, false);

        assert_eq!(
            store
                .edit_review_record("review-record-hsbc-cash", 1, None, Some("-750.00"), None)
                .expect("reject negative amount value"),
            review_conflict("invalid_review_edit")
        );
        assert_eq!(
            store
                .edit_review_record("review-record-hsbc-cash", 1, None, None, Some("-750.00"))
                .expect("accept signed balance delta")
                .status,
            ReviewMutationStatus::Updated
        );
    }

    #[test]
    fn records_a_safe_failure_for_a_claimed_review_batch() {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_review_repayment(&mut store, false);
        let job = store
            .enqueue_commit_review_batch(&["review-record-hsbc-cash".to_owned()])
            .expect("enqueue batch");
        let claimed = store
            .claim_review_batch(&job.job_id, "test-worker")
            .expect("claim batch")
            .expect("queued job claims once");

        let failed = store
            .fail_review_batch(&claimed, "review_core_failed")
            .expect("persist safe job failure");

        assert_eq!(failed.status, ReviewJobStatus::Failed);
        assert!(failed.outcomes.is_empty());
        let error_json: String = store
            .connection
            .query_row(
                "SELECT error_json FROM jobs WHERE id = ?1",
                [&job.job_id],
                |row| row.get(0),
            )
            .expect("read persisted safe error");
        assert_eq!(error_json, r#"{"errorCode":"review_core_failed"}"#);
    }
}
