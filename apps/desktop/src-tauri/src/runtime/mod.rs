use crate::database::imports::{SourceCapturePlan, SourceDocumentReadPlan};
use crate::database::intake::{
    IntakeAcquisitionChannel, IntakeBatchInput, IntakeBatchItemInput, IntakeItemFinalization,
    IntakeRejectionKind, bounded_safe_input_label,
};
use crate::database::restore_decisions::RestoreDecisionState;
use crate::{
    database::{
        AccountConfirmationOutcome, AccountConfirmationPrompt, CandidateAccountDecisionInput,
        ClaimedReviewBatch, CommitReviewGroup, CorePreparedReversalEvent, CorePreparedReviewEvent,
        CoreReviewRecord, DATABASE_FILE_NAME, ManualImportStore, MoneyOverview, ParseDocumentClaim,
        ParseDocumentJob, RecentActivitySummary, RelationshipCandidateSummary,
        ReviewBatchGroupOutcome, ReviewBatchGroupStatus, ReviewItemDetail, ReviewItemSummary,
        ReviewJobSummary, ReviewMutationOutcome, ReviewMutationStatus,
        ReviewRelationshipCandidateInput, SourceDocumentImport, SourceDocumentImportOutcome,
        SourceDocumentImportStatus, SourceDocumentRoutingOutcome, SourceDocumentView,
        StatementPasswordStatus, TrustedAccountCandidate, TrustedDocumentClassification,
        UndoOutcome, ValidatedExternalRecordInput, ValidatedStructuredParseInput,
    },
    local_inbox::{
        AuthorizedRoot, BACKUPS_DIRECTORY_NAME, BookmarkResolution, CaptureDeferReason,
        CaptureOutcome, FileSnapshot, LocalInboxPaths, NativePreflight, SETTLE_INTERVAL,
        SystemNativePreflight, authorize_root, capture_after_second_scan, ensure_inbox_paths,
        first_snapshot_after_preflight, resolve_root_bookmark,
    },
    source_observations::{ExtractionBundle, SourceObservationKind, extract_bundle},
    vault::{
        create_password_wrapper, create_recovery_file, open_password_wrapper,
        password_wrapper_profile, recovery_file_fingerprint,
    },
    viewer::{
        PdfAccess, RenderedDocumentPage, pdf_access, render_image_document,
        render_pdf_page_with_password,
    },
};
#[cfg(target_os = "macos")]
use apple_native_keyring_store::keychain::{Cred as KeychainCredential, MacKeychainDomain};
use keyring_core::{Entry as KeyringEntry, Error as KeyringError};
use rand::{RngCore, rngs::OsRng};
#[cfg(target_os = "macos")]
use security_framework::{
    item::{ItemClass, ItemSearchOptions},
    os::macos::keychain::{SecKeychain, SecPreferencesDomain},
};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(all(test, unix))]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{self, Write},
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_shell::{
    ShellExt,
    process::{CommandChild, CommandEvent},
};
use tokio::time::{Instant, timeout_at};
use zeroize::Zeroizing;

const KEY_LEN: usize = 32;
const KEY_FILE_NAME: &str = "vault-key.ccenv";
const RECOVERY_STATUS_FILE_NAME: &str = "vault-recovery.status";
const RECOVERY_STATUS_MAGIC: &[u8; 8] = b"CCRECST1";
const RECOVERY_STATUS_V2_MAGIC: &[u8; 8] = b"CCRECST2";
const KEYCHAIN_ACCOUNT: &str = "active-vault";
const KEYCHAIN_ITEM_NOT_FOUND_STATUS: i32 = -25300;
const KEYCHAIN_ITEM_INVALIDATED_STATUS: i32 = -25301;
const KEYCHAIN_SERVICE: &str = "dev.cancan.desktop.remembered-vault";
const STATEMENT_PASSWORD_KEYCHAIN_SERVICE: &str = "dev.cancan.desktop.statement-password";
const GMAIL_REFRESH_TOKEN_KEYCHAIN_SERVICE: &str = "dev.cancan.desktop.gmail-refresh-token";
const LOCAL_INBOX_BOOKMARK_ACCOUNT: &str = "authorized-root";
const LOCAL_INBOX_BOOKMARK_KEYCHAIN_SERVICE: &str = "dev.cancan.desktop.local-inbox";
const IMPORT_POLICY_VERSION: &str = "manual-import-v1";
const NORMALIZER_TIMEOUT: Duration = Duration::from_secs(10);
const NORMALIZER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const NORMALIZER_MAX_MESSAGE_BYTES: usize = 256 * 1024;
// A CSV preview returns at most the first lines of the decrypted text. A small
// CSV may appear in full, but the renderer never receives raw original-file
// bytes or unbounded content. The caps keep IPC bounded while giving enough
// context to recognize a statement export.
const PREVIEW_MAX_LINES: usize = 200;
const PREVIEW_MAX_BYTES: usize = 32 * 1024;
type DocumentPasswordSessions = HashMap<String, Zeroizing<Vec<u8>>>;

#[derive(Debug)]
pub(super) struct ExtractedDocument {
    pub(super) bundle: ExtractionBundle,
    pub(super) vault_session_generation: u64,
}

impl Deref for ExtractedDocument {
    type Target = ExtractionBundle;

    fn deref(&self) -> &Self::Target {
        &self.bundle
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VaultStatus {
    NotCreated,
    Locked,
    Unlocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SavedStatementPasswordResult {
    Invalid,
    Unavailable,
    Unlocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VaultAccessStatus {
    recovery_configured: bool,
    remembered_on_this_mac: Option<bool>,
    status: VaultStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocalInboxAccessState {
    Disabled,
    Enabled,
    NeedsAttention,
    NeedsReauthorization,
    Paused,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalInboxScanSummary {
    already_present: u64,
    deferred: u64,
    imported: u64,
    suppressed: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalInboxStatus {
    access_state: LocalInboxAccessState,
    backups_prepared: bool,
    enabled: bool,
    inbox_label: &'static str,
    last_scan: Option<LocalInboxScanSummary>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) enum SourceDocumentStatus {
    FileDeleted,
    Missing,
    NeedsAttention,
    Processing,
    Ready,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct SourceDocumentSummary {
    attention_reason: Option<String>,
    byte_size: u64,
    document_status: SourceDocumentStatus,
    document_id: String,
    file_state: String,
    mime_type: String,
    original_filename: String,
    received_at: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoneySourceSummary {
    display_name: String,
    money_source_id: String,
    source_type: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatementPasswordSourceSummary {
    display_name: String,
    has_saved_password: bool,
    money_source_id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceDocumentPreview {
    line_count: u64,
    preview_lines: u64,
    preview_text: String,
    truncated: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerAccount {
    account_type: String,
    currency: Option<String>,
    masked_identifier: Option<String>,
    proposal_account_id: String,
    provider_account_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerDocument {
    document_type: String,
    provider_key: String,
    provider_root_id: Option<String>,
    statement_id: Option<String>,
    statement_period: Option<NormalizerStatementPeriod>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerStatementPeriod {
    from: Option<String>,
    to: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    tag = "status",
    deny_unknown_fields
)]
enum NormalizerResult {
    Classified {
        profile: Box<NormalizerProfile>,
        proposal: Box<NormalizerProposal>,
        /// Sidecar-echoed canonical document key. The host re-derives this
        /// value in `review.rs` and rejects mismatches; it never reaches
        /// persistence.
        semantic_document_key: String,
    },
    NeedsAttention {
        reason: String,
    },
}

/// Canonical semantic document key. Byte-identical to the TypeScript
/// `semanticDocumentKey` in `packages/parsers/src/validate-structured-proposal.ts`:
/// `providerKey` + (`:${providerRootId}` when present) + `:${statementId}`.
pub(super) fn derive_semantic_document_key(
    provider_key: &str,
    provider_root_id: Option<&str>,
    statement_id: &str,
) -> String {
    match provider_root_id {
        Some(root_id) => format!("{provider_key}:{root_id}:{statement_id}"),
        None => format!("{provider_key}:{statement_id}"),
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerProfile {
    document_type: String,
    extraction_engines: Vec<NormalizerProfileExtractionEngine>,
    id: String,
    input_strategy: String,
    model: String,
    model_provider: String,
    normalizer_runtime: String,
    ocr_engines: Vec<NormalizerProfileOcrEngine>,
    package_id: String,
    package_version: String,
    parser_version: String,
    prompt_version: String,
    provider_key: String,
    review_only: bool,
    schema_version: String,
    skill_version: String,
    tool_contract_version: String,
    validator_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerProfileExtractionEngine {
    engine: String,
    kind: NormalizerProfileExtractionKind,
    version: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerProfileOcrEngine {
    engine: String,
    version: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum NormalizerProfileExtractionKind {
    NativeText,
    TableCell,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerProposal {
    accounts: Vec<NormalizerAccount>,
    closing_snapshots: Vec<NormalizerRecord>,
    document: NormalizerDocument,
    opening_snapshots: Vec<NormalizerRecord>,
    records: Vec<NormalizerRecord>,
    status: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerMoney {
    currency: String,
    value: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerRecordValidation {
    deterministic_validation_passed: bool,
    raw_grounded: bool,
    schema_valid: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NormalizerRecord {
    account_balance_delta: Option<NormalizerMoney>,
    amount: Option<NormalizerMoney>,
    balance_after: Option<NormalizerMoney>,
    description_normalized: Option<String>,
    description_raw: Option<String>,
    event_type: Option<String>,
    instrument_symbol: Option<String>,
    posted_at: Option<String>,
    posted_on: Option<String>,
    posting_status: Option<String>,
    proposal_account_id: Option<String>,
    proposal_record_id: String,
    provider_record_id: Option<String>,
    quantity: Option<String>,
    raw: serde_json::Value,
    record_type: String,
    stable_record_key: String,
    statement_entry_side: Option<String>,
    transaction_on: Option<String>,
    validation: NormalizerRecordValidation,
    valuation: Option<NormalizerMoney>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NormalizerCommand<'a> {
    document_id: &'a str,
    extraction_bundle: &'a ExtractionBundle,
    request_id: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum NormalizerMessage {
    Ready {
        #[serde(rename = "protocolVersion")]
        protocol_version: u8,
        runtime: String,
        #[serde(rename = "environmentCleared")]
        environment_cleared: bool,
    },
    Result {
        #[serde(rename = "requestId")]
        request_id: String,
        result: NormalizerResult,
    },
    Error {
        code: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewCoreCommand<'a, T> {
    input: &'a T,
    operation: &'static str,
    request_id: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RelationshipCandidatesInput<'a> {
    candidates: &'a [CoreReviewRecord],
    event_type: &'a str,
    record: &'a CoreReviewRecord,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RelationshipPreparationInput<'a> {
    event_type: &'a str,
    records: &'a [CoreReviewRecord; 2],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReversalPreparationInput<'a> {
    event: &'a CorePreparedReviewEvent,
    event_date: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CoreCandidate {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ReviewCoreReadyEvent {
    Relationship(CorePreparedReviewEvent),
    Reversal(CorePreparedReversalEvent),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status", deny_unknown_fields)]
enum ReviewCoreResult {
    Candidates { candidates: Vec<CoreCandidate> },
    Ready { event: ReviewCoreReadyEvent },
    Review { reasons: Vec<String> },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ReviewCoreMessage {
    Ready {
        #[serde(rename = "protocolVersion")]
        protocol_version: u8,
        runtime: String,
        #[serde(rename = "environmentCleared")]
        environment_cleared: bool,
    },
    Result {
        #[serde(rename = "requestId")]
        request_id: String,
        result: ReviewCoreResult,
    },
    Error {
        code: String,
    },
}

#[derive(Clone)]
pub(crate) struct VaultRuntime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    document_passwords: Mutex<DocumentPasswordSessions>,
    gmail_refresh_tokens: Arc<dyn GmailRefreshTokenStore>,
    local_inbox_access: Mutex<Option<AuthorizedRoot>>,
    local_inbox_bookmarks: Arc<dyn LocalInboxBookmarkStore>,
    local_inbox_last_scan: Mutex<Option<LocalInboxScanSummary>>,
    local_inbox_scan_guard: Mutex<()>,
    local_inbox_watcher: Mutex<Option<notify::RecommendedWatcher>>,
    local_inbox_needs_attention: AtomicBool,
    local_inbox_needs_reauthorization: AtomicBool,
    remembered_keys: Arc<dyn RememberedKeyStore>,
    root: PathBuf,
    statement_passwords: Arc<dyn StatementPasswordStore>,
    store: Mutex<Option<ManualImportStore>>,
    vault_session_generation: AtomicU64,
    #[cfg(test)]
    intake_test_hooks: Mutex<IntakeTestHooks>,
}

#[cfg(test)]
#[derive(Default)]
struct IntakeTestHooks {
    fail_next_plan: bool,
    fail_next_persist: bool,
    fail_next_finalization: bool,
    fail_observation_filename: Option<String>,
    local_inbox_scan_barrier: Option<Arc<std::sync::Barrier>>,
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum IntakeTestFault {
    Plan,
    Persist,
    Finalization,
}

mod accounts;
mod documents;
mod documents_intake;
mod error;
mod gmail;
mod gmail_connector;
#[cfg(test)]
mod gmail_tests;
#[cfg(test)]
mod hardening_tests;
mod inbox;
mod inbox_intake;
mod inbox_watcher;
#[cfg(test)]
mod intake_tests;
mod keyring;
#[cfg(test)]
mod keyring_tests;
mod lifecycle;
#[cfg(test)]
mod remembered_key_tests;
mod review;
mod sidecar;
mod source_confirmation;
mod tasks;
#[cfg(test)]
mod tasks_test_support;
#[cfg(test)]
mod tasks_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
mod undo;
mod vault_lifecycle;

pub(crate) use accounts::*;
pub(crate) use documents::*;
pub(crate) use error::*;
pub(crate) use inbox::*;
use keyring::*;
pub(crate) use lifecycle::*;
pub(crate) use review::*;
use sidecar::*;
pub(crate) use source_confirmation::*;
pub(crate) use tasks::*;
pub(crate) use undo::*;
pub(crate) use vault_lifecycle::*;

impl VaultRuntime {
    pub(super) fn store(&self) -> Result<MutexGuard<'_, Option<ManualImportStore>>, RuntimeError> {
        self.inner
            .store
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))
    }

    #[cfg(test)]
    pub(super) fn store_for_vault_session(
        &self,
        vault_session_generation: u64,
    ) -> Result<MutexGuard<'_, Option<ManualImportStore>>, RuntimeError> {
        let store = self.store()?;
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != vault_session_generation {
            return Err(RuntimeError::new("vault_locked"));
        }
        Ok(store)
    }

    #[cfg(test)]
    fn take_intake_test_fault(&self, fault: IntakeTestFault) -> bool {
        let Ok(mut hooks) = self.inner.intake_test_hooks.lock() else {
            return false;
        };
        match fault {
            IntakeTestFault::Plan => std::mem::take(&mut hooks.fail_next_plan),
            IntakeTestFault::Persist => std::mem::take(&mut hooks.fail_next_persist),
            IntakeTestFault::Finalization => std::mem::take(&mut hooks.fail_next_finalization),
        }
    }

    #[cfg(test)]
    fn take_observation_test_fault(&self, path: &Path) -> bool {
        let Some(filename) = path.file_name().and_then(|value| value.to_str()) else {
            return false;
        };
        let Ok(mut hooks) = self.inner.intake_test_hooks.lock() else {
            return false;
        };
        hooks
            .fail_observation_filename
            .as_deref()
            .is_some_and(|expected| expected == filename)
            && hooks.fail_observation_filename.take().is_some()
    }

    #[cfg(test)]
    fn wait_for_local_inbox_scan_test_barrier(&self) {
        let barrier = self
            .inner
            .intake_test_hooks
            .lock()
            .ok()
            .and_then(|hooks| hooks.local_inbox_scan_barrier.clone());
        if let Some(barrier) = barrier {
            barrier.wait();
        }
    }

    #[cfg(test)]
    pub(super) fn inject_intake_plan_failure(&self) {
        if let Ok(mut hooks) = self.inner.intake_test_hooks.lock() {
            hooks.fail_next_plan = true;
        }
    }

    #[cfg(test)]
    pub(super) fn inject_intake_persist_failure(&self) {
        if let Ok(mut hooks) = self.inner.intake_test_hooks.lock() {
            hooks.fail_next_persist = true;
        }
    }

    #[cfg(test)]
    pub(super) fn inject_intake_finalization_failure(&self) {
        if let Ok(mut hooks) = self.inner.intake_test_hooks.lock() {
            hooks.fail_next_finalization = true;
        }
    }

    #[cfg(test)]
    pub(super) fn inject_observation_failure(&self, filename: &str) {
        if let Ok(mut hooks) = self.inner.intake_test_hooks.lock() {
            hooks.fail_observation_filename = Some(filename.to_owned());
        }
    }

    #[cfg(test)]
    pub(super) fn inject_local_inbox_scan_barrier(&self, barrier: Arc<std::sync::Barrier>) {
        if let Ok(mut hooks) = self.inner.intake_test_hooks.lock() {
            hooks.local_inbox_scan_barrier = Some(barrier);
        }
    }

    #[cfg(test)]
    pub(super) fn clear_local_inbox_scan_barrier(&self) {
        if let Ok(mut hooks) = self.inner.intake_test_hooks.lock() {
            hooks.local_inbox_scan_barrier = None;
        }
    }

    pub(super) fn document_passwords(
        &self,
    ) -> Result<MutexGuard<'_, DocumentPasswordSessions>, RuntimeError> {
        self.inner
            .document_passwords
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))
    }

    pub(super) fn load_remembered_master_key(
        &self,
    ) -> Result<Option<Zeroizing<[u8; KEY_LEN]>>, ()> {
        let secret = match self.inner.remembered_keys.load() {
            Ok(Some(secret)) if secret.len() == KEY_LEN => secret,
            Ok(_) => {
                // The protected key is gone or malformed; remove the presence
                // marker so the UI stops offering Touch ID.
                self.inner.remembered_keys.delete()?;
                return Ok(None);
            }
            Err(()) if self.inner.remembered_keys.invalidated() => {
                // The enrolled Touch ID fingerprint set changed and invalidated
                // the item; clean up both items so the UI stops offering a
                // Touch ID unlock that can never succeed. Other load errors
                // (user cancel, authentication unavailable) keep the marker so
                // the retry stays possible.
                self.inner.remembered_keys.delete()?;
                return Ok(None);
            }
            Err(()) => return Err(()),
        };
        Ok(Some(Zeroizing::new(
            <[u8; KEY_LEN]>::try_from(secret.as_slice()).expect("length checked"),
        )))
    }
}
