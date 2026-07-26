use crate::{
    database::{
        ClaimedReviewBatch, CommitReviewGroup, CorePreparedReversalEvent, CorePreparedReviewEvent,
        CoreReviewRecord, DATABASE_FILE_NAME, ManualImportStore, MoneyOverview,
        RecentActivitySummary, RelationshipCandidateSummary, ReviewBatchGroupOutcome,
        ReviewBatchGroupStatus, ReviewItemDetail, ReviewItemSummary, ReviewJobSummary,
        ReviewMutationOutcome, ReviewMutationStatus, ReviewRelationshipCandidateInput,
        SourceDocumentImport, SourceDocumentImportOutcome, SourceDocumentImportStatus,
        SourceDocumentRoutingOutcome, SourceDocumentView, StatementCoverageDecision,
        StatementCoverageDecisionInput, StatementCoveragePolicy, StatementCoveragePrompt,
        StatementPasswordStatus, TrustedAccountCandidate, TrustedDocumentClassification,
        UndoOutcome,
    },
    local_inbox::{
        AuthorizedRoot, BACKUPS_DIRECTORY_NAME, BookmarkResolution, CaptureOutcome,
        LocalInboxPaths, NativePreflight, SETTLE_INTERVAL, SystemNativePreflight, authorize_root,
        capture_after_second_scan, ensure_inbox_paths, first_snapshot_after_preflight,
        resolve_root_bookmark,
    },
    source_observations::{ExtractionBundle, extract_bundle},
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
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
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
const KEYCHAIN_ACCOUNT: &str = "active-vault";
const KEYCHAIN_ITEM_NOT_FOUND_STATUS: i32 = -25300;
const KEYCHAIN_SERVICE: &str = "dev.cancan.desktop.remembered-vault";
const STATEMENT_PASSWORD_KEYCHAIN_SERVICE: &str = "dev.cancan.desktop.statement-password";
const LOCAL_INBOX_BOOKMARK_ACCOUNT: &str = "authorized-root";
const LOCAL_INBOX_BOOKMARK_KEYCHAIN_SERVICE: &str = "dev.cancan.desktop.local-inbox";
const IMPORT_POLICY_VERSION: &str = "manual-import-v1";
const NORMALIZER_TIMEOUT: Duration = Duration::from_secs(10);
const NORMALIZER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
// The shipped synthetic normalizer models an on-demand export, so it has no
// statement cadence. Real provider packages must add an explicit declaration;
// CanCan never infers cadence from filenames or prior dates.
const STATEMENT_COVERAGE_POLICIES: &[StatementCoveragePolicy<'static>] = &[];
// A CSV preview returns at most the first lines of the decrypted text. A small
// CSV may appear in full, but the renderer never receives raw original-file
// bytes or unbounded content. The caps keep IPC bounded while giving enough
// context to recognize a statement export.
const PREVIEW_MAX_LINES: usize = 200;
const PREVIEW_MAX_BYTES: usize = 32 * 1024;
type DocumentPasswordSessions = HashMap<String, Zeroizing<Vec<u8>>>;

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StatementCoverageDecisionAction {
    NotExpected,
    RemindLater,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatementCoverageDecisionRequest {
    account_id: String,
    action: StatementCoverageDecisionAction,
    document_type: String,
    money_source_id: String,
    remind_after: Option<String>,
    statement_period_from: String,
    statement_period_to: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceDocumentSummary {
    byte_size: u64,
    document_status: &'static str,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NormalizerAccount {
    account_type: String,
    currency: Option<String>,
    masked_identifier: Option<String>,
    provider_account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NormalizerDocument {
    document_type: String,
    provider_key: String,
    statement_id: Option<String>,
    statement_period: Option<NormalizerStatementPeriod>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NormalizerStatementPeriod {
    from: Option<String>,
    to: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
enum NormalizerResult {
    Classified { proposal: NormalizerProposal },
    NeedsAttention { reason: String },
}

#[derive(Debug, Deserialize)]
struct NormalizerProposal {
    accounts: Vec<NormalizerAccount>,
    document: NormalizerDocument,
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
#[serde(tag = "type", rename_all = "snake_case")]
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
    Error,
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
    local_inbox_access: Mutex<Option<AuthorizedRoot>>,
    local_inbox_bookmarks: Arc<dyn LocalInboxBookmarkStore>,
    local_inbox_last_scan: Mutex<Option<LocalInboxScanSummary>>,
    local_inbox_needs_attention: AtomicBool,
    local_inbox_needs_reauthorization: AtomicBool,
    remembered_keys: Arc<dyn RememberedKeyStore>,
    root: PathBuf,
    statement_passwords: Arc<dyn StatementPasswordStore>,
    store: Mutex<Option<ManualImportStore>>,
    system_lock_generation: AtomicU64,
    system_session_active: AtomicBool,
    vault_session_generation: AtomicU64,
}

mod documents;
mod error;
mod inbox;
mod keyring;
mod review;
mod sidecar;
#[cfg(test)]
mod tests;
mod vault_lifecycle;

pub(crate) use documents::*;
pub(crate) use error::*;
pub(crate) use inbox::*;
use keyring::*;
pub(crate) use review::*;
use sidecar::*;
pub(crate) use vault_lifecycle::*;

impl VaultRuntime {
    pub(super) fn store(&self) -> Result<MutexGuard<'_, Option<ManualImportStore>>, RuntimeError> {
        let system_lock_generation = self.inner.system_lock_generation.load(Ordering::SeqCst);
        self.store_for_system_generation(system_lock_generation)
    }

    pub(super) fn store_for_system_generation(
        &self,
        system_lock_generation: u64,
    ) -> Result<MutexGuard<'_, Option<ManualImportStore>>, RuntimeError> {
        if !self.system_session_active() {
            return Err(RuntimeError::new("vault_locked"));
        }
        let store = self.raw_store()?;
        if !self.system_session_active()
            || self.inner.system_lock_generation.load(Ordering::SeqCst) != system_lock_generation
        {
            return Err(RuntimeError::new("vault_locked"));
        }
        Ok(store)
    }

    pub(super) fn raw_store(
        &self,
    ) -> Result<MutexGuard<'_, Option<ManualImportStore>>, RuntimeError> {
        self.inner
            .store
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))
    }

    pub(super) fn document_passwords(
        &self,
    ) -> Result<MutexGuard<'_, DocumentPasswordSessions>, RuntimeError> {
        self.inner
            .document_passwords
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))
    }

    pub(super) fn system_session_active(&self) -> bool {
        self.inner.system_session_active.load(Ordering::SeqCst)
    }

    pub(super) fn load_remembered_master_key(
        &self,
    ) -> Result<Option<Zeroizing<[u8; KEY_LEN]>>, ()> {
        let Some(secret) = self.inner.remembered_keys.load()? else {
            return Ok(None);
        };
        let master_key = match secret.as_slice().try_into() {
            Ok(master_key) => master_key,
            Err(_) => {
                self.inner.remembered_keys.delete()?;
                return Ok(None);
            }
        };
        Ok(Some(Zeroizing::new(master_key)))
    }
}
