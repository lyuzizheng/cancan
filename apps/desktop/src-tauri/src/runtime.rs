use crate::{
    database::{
        DATABASE_FILE_NAME, ManualImportStore, SourceDocumentFileInput, SourceDocumentImport,
        SourceDocumentImportOutcome, SourceDocumentImportStatus, SourceDocumentRoutingOutcome,
        SourceDocumentView, StatementPasswordStatus, TrustedAccountCandidate,
        TrustedDocumentClassification,
    },
    vault::{
        create_password_wrapper, create_recovery_file, open_password_wrapper,
        password_wrapper_profile, recovery_file_fingerprint,
    },
    viewer::{PdfAccess, RenderedDocumentPage, pdf_access, render_pdf_page_with_password},
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
const IMPORT_POLICY_VERSION: &str = "manual-import-v1";
const NORMALIZER_TIMEOUT: Duration = Duration::from_secs(10);
const NORMALIZER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
// A non-PDF preview returns only the first lines of the decrypted text, never
// the complete original file bytes. The caps give enough context to recognize
// a statement export while keeping the renderer boundary and IPC bounded,
// mirroring the PDF viewer's bounded-pixel budget.
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

#[derive(Clone, Debug, Serialize)]
pub(crate) struct VaultCommandError {
    code: &'static str,
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
    content: &'a str,
    document_id: &'a str,
    mime_type: &'a str,
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

impl VaultCommandError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }
}

#[derive(Debug)]
pub(crate) struct RuntimeError {
    code: &'static str,
}

impl RuntimeError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }

    #[cfg(test)]
    pub(crate) fn code(&self) -> &'static str {
        self.code
    }
}

impl From<RuntimeError> for VaultCommandError {
    fn from(error: RuntimeError) -> Self {
        Self::new(error.code)
    }
}

#[derive(Clone)]
pub(crate) struct VaultRuntime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    document_passwords: Mutex<DocumentPasswordSessions>,
    remembered_keys: Arc<dyn RememberedKeyStore>,
    root: PathBuf,
    statement_passwords: Arc<dyn StatementPasswordStore>,
    store: Mutex<Option<ManualImportStore>>,
    system_lock_generation: AtomicU64,
    system_session_active: AtomicBool,
    vault_session_generation: AtomicU64,
}

trait RememberedKeyStore: Send + Sync {
    fn delete(&self) -> Result<(), ()>;
    fn is_present(&self) -> Result<bool, ()>;
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()>;
    fn save(&self, secret: &[u8]) -> Result<(), ()>;
}

trait StatementPasswordStore: Send + Sync {
    fn delete(&self, secret_ref: &str) -> Result<(), ()>;
    fn load(&self, secret_ref: &str) -> Result<Option<Zeroizing<Vec<u8>>>, ()>;
    fn save(&self, secret_ref: &str, secret: &[u8]) -> Result<(), ()>;
}

#[derive(Clone)]
struct KeychainRememberedKeyStore {
    account: String,
    service: String,
}

impl KeychainRememberedKeyStore {
    fn production() -> Self {
        Self::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
    }

    fn new(service: impl Into<String>, account: impl Into<String>) -> Self {
        Self {
            account: account.into(),
            service: service.into(),
        }
    }

    #[cfg(target_os = "macos")]
    fn entry(&self) -> Result<KeyringEntry, ()> {
        KeychainCredential::build(MacKeychainDomain::User, &self.service, &self.account)
            .map_err(|_| ())
    }

    #[cfg(not(target_os = "macos"))]
    fn entry(&self) -> Result<KeyringEntry, ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn item_query(&self) -> Result<ItemSearchOptions, ()> {
        let keychain =
            SecKeychain::default_for_domain(SecPreferencesDomain::User).map_err(|_| ())?;
        let mut query = ItemSearchOptions::new();
        query
            .keychains(&[keychain])
            .class(ItemClass::generic_password())
            .service(&self.service)
            .account(&self.account)
            .limit(1);
        Ok(query)
    }
}

impl RememberedKeyStore for KeychainRememberedKeyStore {
    #[cfg(target_os = "macos")]
    fn delete(&self) -> Result<(), ()> {
        match self.item_query()?.delete() {
            Ok(()) => Ok(()),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(()),
            Err(_) => Err(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn delete(&self) -> Result<(), ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn is_present(&self) -> Result<bool, ()> {
        match self.item_query()?.search() {
            Ok(_) => Ok(true),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(false),
            Err(_) => Err(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn is_present(&self) -> Result<bool, ()> {
        Err(())
    }

    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        match self.entry()?.get_secret() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(_) => Err(()),
        }
    }

    fn save(&self, secret: &[u8]) -> Result<(), ()> {
        self.entry()?.set_secret(secret).map_err(|_| ())
    }
}

#[derive(Clone)]
struct KeychainStatementPasswordStore {
    service: String,
}

impl KeychainStatementPasswordStore {
    fn production() -> Self {
        Self::new(STATEMENT_PASSWORD_KEYCHAIN_SERVICE)
    }

    fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    #[cfg(target_os = "macos")]
    fn entry(&self, secret_ref: &str) -> Result<KeyringEntry, ()> {
        KeychainCredential::build(MacKeychainDomain::User, &self.service, secret_ref)
            .map_err(|_| ())
    }

    #[cfg(not(target_os = "macos"))]
    fn entry(&self, _secret_ref: &str) -> Result<KeyringEntry, ()> {
        Err(())
    }

    #[cfg(target_os = "macos")]
    fn item_query(&self, secret_ref: &str) -> Result<ItemSearchOptions, ()> {
        let keychain =
            SecKeychain::default_for_domain(SecPreferencesDomain::User).map_err(|_| ())?;
        let mut query = ItemSearchOptions::new();
        query
            .keychains(&[keychain])
            .class(ItemClass::generic_password())
            .service(&self.service)
            .account(secret_ref)
            .limit(1);
        Ok(query)
    }
}

impl StatementPasswordStore for KeychainStatementPasswordStore {
    #[cfg(target_os = "macos")]
    fn delete(&self, secret_ref: &str) -> Result<(), ()> {
        match self.item_query(secret_ref)?.delete() {
            Ok(()) => Ok(()),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND_STATUS => Ok(()),
            Err(_) => Err(()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn delete(&self, _secret_ref: &str) -> Result<(), ()> {
        Err(())
    }

    fn load(&self, secret_ref: &str) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
        match self.entry(secret_ref)?.get_secret() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(_) => Err(()),
        }
    }

    fn save(&self, secret_ref: &str, secret: &[u8]) -> Result<(), ()> {
        self.entry(secret_ref)?.set_secret(secret).map_err(|_| ())
    }
}

impl VaultRuntime {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self::with_secret_stores(
            root,
            Arc::new(KeychainRememberedKeyStore::production()),
            Arc::new(KeychainStatementPasswordStore::production()),
        )
    }

    #[cfg(test)]
    fn with_remembered_keys(root: PathBuf, remembered_keys: Arc<dyn RememberedKeyStore>) -> Self {
        Self::with_secret_stores(
            root,
            remembered_keys,
            Arc::new(KeychainStatementPasswordStore::production()),
        )
    }

    fn with_secret_stores(
        root: PathBuf,
        remembered_keys: Arc<dyn RememberedKeyStore>,
        statement_passwords: Arc<dyn StatementPasswordStore>,
    ) -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                document_passwords: Mutex::new(HashMap::new()),
                remembered_keys,
                root,
                statement_passwords,
                store: Mutex::new(None),
                system_lock_generation: AtomicU64::new(0),
                system_session_active: AtomicBool::new(true),
                vault_session_generation: AtomicU64::new(0),
            }),
        }
    }

    pub(crate) fn access_status(&self) -> Result<VaultAccessStatus, RuntimeError> {
        let status = self.status()?;
        let remembered_on_this_mac = if status == VaultStatus::NotCreated {
            Some(false)
        } else {
            self.inner.remembered_keys.is_present().ok()
        };
        Ok(VaultAccessStatus {
            recovery_configured: self.recovery_configured(),
            remembered_on_this_mac,
            status,
        })
    }

    fn recovery_configured(&self) -> bool {
        let Ok(status) = fs::read(self.inner.root.join(RECOVERY_STATUS_FILE_NAME)) else {
            return false;
        };
        status.len() == RECOVERY_STATUS_MAGIC.len() + KEY_LEN
            && &status[..RECOVERY_STATUS_MAGIC.len()] == RECOVERY_STATUS_MAGIC
    }

    pub(crate) fn status(&self) -> Result<VaultStatus, RuntimeError> {
        if !self.system_session_active() {
            return self.locked_status();
        }
        let store = self.store()?;
        if store.is_some() {
            return Ok(VaultStatus::Unlocked);
        }
        self.locked_status()
    }

    fn locked_status(&self) -> Result<VaultStatus, RuntimeError> {
        if !self.inner.root.exists() {
            return Ok(VaultStatus::NotCreated);
        }
        if !self.inner.root.join(DATABASE_FILE_NAME).is_file() {
            return Err(RuntimeError::new("invalid_vault"));
        }
        let wrapper = fs::read(self.inner.root.join(KEY_FILE_NAME))
            .map_err(|_| RuntimeError::new("invalid_vault"))?;
        password_wrapper_profile(&wrapper).map_err(|_| RuntimeError::new("invalid_vault"))?;
        Ok(VaultStatus::Locked)
    }

    pub(crate) fn create(&self, password: &[u8]) -> Result<VaultStatus, RuntimeError> {
        if password.is_empty() {
            return Err(RuntimeError::new("password_required"));
        }
        let mut store = self.store()?;
        if self.inner.root.exists() {
            return Err(RuntimeError::new("vault_already_exists"));
        }
        let parent = self
            .inner
            .root
            .parent()
            .ok_or_else(|| RuntimeError::new("vault_create_failed"))?;
        fs::create_dir_all(parent).map_err(|_| RuntimeError::new("vault_create_failed"))?;

        let candidate = parent.join(candidate_name());
        fs::create_dir(&candidate).map_err(|_| RuntimeError::new("vault_create_failed"))?;
        let result = self.create_candidate(&candidate, parent, password);
        if result.is_err() && candidate.exists() && fs::remove_dir_all(&candidate).is_ok() {
            let _ = sync_directory(parent);
        }
        let opened = result?;
        *store = Some(opened);
        self.advance_vault_session();
        Ok(VaultStatus::Unlocked)
    }

    fn create_candidate(
        &self,
        candidate: &Path,
        parent: &Path,
        password: &[u8],
    ) -> Result<ManualImportStore, RuntimeError> {
        let mut master_key = Zeroizing::new([0_u8; KEY_LEN]);
        OsRng.fill_bytes(master_key.as_mut());
        let wrapper = create_password_wrapper(password, &master_key)
            .map_err(|_| RuntimeError::new("vault_create_failed"))?;

        let candidate_store = ManualImportStore::open(candidate, Zeroizing::new(*master_key))
            .map_err(|_| RuntimeError::new("vault_create_failed"))?;
        drop(candidate_store);
        write_new_synced(&candidate.join(KEY_FILE_NAME), &wrapper)
            .map_err(|_| RuntimeError::new("vault_create_failed"))?;
        sync_directory(candidate).map_err(|_| RuntimeError::new("vault_create_failed"))?;
        fs::rename(candidate, &self.inner.root)
            .map_err(|_| RuntimeError::new("vault_create_failed"))?;
        if sync_directory(parent).is_err() {
            return match self.rollback_activation(candidate, parent) {
                Ok(()) => Err(RuntimeError::new("vault_create_failed")),
                Err(()) => Err(RuntimeError::new("invalid_vault")),
            };
        }

        match ManualImportStore::open_existing(&self.inner.root, master_key) {
            Ok(store) => Ok(store),
            Err(_) => match self.rollback_activation(candidate, parent) {
                Ok(()) => Err(RuntimeError::new("vault_create_failed")),
                Err(()) => Err(RuntimeError::new("invalid_vault")),
            },
        }
    }

    fn rollback_activation(&self, candidate: &Path, parent: &Path) -> Result<(), ()> {
        fs::rename(&self.inner.root, candidate).map_err(|_| ())?;
        sync_directory(parent).map_err(|_| ())
    }

    pub(crate) fn unlock(&self, password: &[u8]) -> Result<VaultStatus, RuntimeError> {
        if password.is_empty() {
            return Err(RuntimeError::new("password_required"));
        }
        let mut store = self.store()?;
        if store.is_some() {
            return Ok(VaultStatus::Unlocked);
        }
        let wrapper = match fs::read(self.inner.root.join(KEY_FILE_NAME)) {
            Ok(wrapper) => wrapper,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return if self.inner.root.exists() {
                    Err(RuntimeError::new("invalid_vault"))
                } else {
                    Err(RuntimeError::new("vault_not_created"))
                };
            }
            Err(_) => return Err(RuntimeError::new("invalid_vault")),
        };
        password_wrapper_profile(&wrapper).map_err(|_| RuntimeError::new("invalid_vault"))?;
        let master_key = open_password_wrapper(&wrapper, password)
            .map_err(|_| RuntimeError::new("invalid_credentials"))?;
        let opened = ManualImportStore::open_existing(&self.inner.root, master_key)
            .map_err(|_| RuntimeError::new("invalid_vault"))?;
        self.reconcile_statement_passwords(&opened)?;
        *store = Some(opened);
        self.advance_vault_session();
        Ok(VaultStatus::Unlocked)
    }

    pub(crate) fn unlock_with_keychain(&self) -> Result<VaultStatus, RuntimeError> {
        let mut store = self.store()?;
        if store.is_some() {
            return Ok(VaultStatus::Unlocked);
        }
        if self.locked_status()? != VaultStatus::Locked {
            return Err(RuntimeError::new("vault_not_created"));
        }
        let Some(master_key) = self
            .load_remembered_master_key()
            .map_err(|_| RuntimeError::new("remembered_unlock_failed"))?
        else {
            return Err(RuntimeError::new("remembered_unlock_unavailable"));
        };
        match ManualImportStore::open_existing(&self.inner.root, master_key) {
            Ok(opened) => {
                self.reconcile_statement_passwords(&opened)?;
                *store = Some(opened);
                self.advance_vault_session();
                Ok(VaultStatus::Unlocked)
            }
            Err(_) => Err(RuntimeError::new("remembered_unlock_failed")),
        }
    }

    pub(crate) fn remember_on_this_mac(&self) -> Result<(), RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        self.inner
            .remembered_keys
            .save(store.master_key())
            .map_err(|_| RuntimeError::new("remember_failed"))
    }

    pub(crate) fn forget_this_mac(&self) -> Result<(), RuntimeError> {
        self.require_unlocked()?;
        self.inner
            .remembered_keys
            .delete()
            .map_err(|_| RuntimeError::new("forget_failed"))
    }

    pub(crate) fn save_recovery_file(&self, destination: &Path) -> Result<(), RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        if self.recovery_configured() {
            return Err(RuntimeError::new("recovery_already_configured"));
        }
        let destination_parent = destination
            .parent()
            .ok_or_else(|| RuntimeError::new("recovery_save_failed"))?
            .canonicalize()
            .map_err(|_| RuntimeError::new("recovery_save_failed"))?;
        let vault_root = self
            .inner
            .root
            .canonicalize()
            .map_err(|_| RuntimeError::new("invalid_vault"))?;
        if destination_parent.starts_with(&vault_root) {
            return Err(RuntimeError::new("recovery_location_invalid"));
        }

        let recovery_file = create_recovery_file(store.master_key())
            .map_err(|_| RuntimeError::new("recovery_create_failed"))?;
        write_atomic(destination, &recovery_file)
            .map_err(|_| RuntimeError::new("recovery_save_failed"))?;

        let mut status = Vec::with_capacity(RECOVERY_STATUS_MAGIC.len() + KEY_LEN);
        status.extend_from_slice(RECOVERY_STATUS_MAGIC);
        status.extend_from_slice(&recovery_file_fingerprint(&recovery_file));
        write_atomic(&self.inner.root.join(RECOVERY_STATUS_FILE_NAME), &status)
            .map_err(|_| RuntimeError::new("recovery_status_failed"))
    }

    pub(crate) fn lock(&self) -> Result<VaultStatus, RuntimeError> {
        self.advance_vault_session();
        let mut store = self.raw_store()?;
        *store = None;
        self.document_passwords()?.clear();
        self.locked_status()
    }

    pub(crate) fn request_system_lock(&self) -> Result<(), RuntimeError> {
        self.inner
            .system_session_active
            .store(false, Ordering::SeqCst);
        self.inner
            .system_lock_generation
            .fetch_add(1, Ordering::SeqCst);
        self.advance_vault_session();
        let mut store = self.raw_store()?;
        *store = None;
        self.document_passwords()?.clear();
        Ok(())
    }

    pub(crate) fn resume_system_session(&self) -> Result<(), RuntimeError> {
        self.advance_vault_session();
        let mut store = self.raw_store()?;
        *store = None;
        self.document_passwords()?.clear();
        self.inner
            .system_session_active
            .store(true, Ordering::SeqCst);
        Ok(())
    }

    pub(crate) fn import_selected_document(
        &self,
        source_path: &Path,
        restore_deleted_document_id: Option<&str>,
    ) -> Result<SourceDocumentImportOutcome, RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let (original_filename, mime_type) = source_document_metadata(source_path)?;
        let document_id = random_identifier("document");
        let audit_id = random_identifier("audit");
        let input = SourceDocumentImport {
            audit_actor: "user",
            audit_id: &audit_id,
            audit_policy_version: IMPORT_POLICY_VERSION,
            audit_reason: "manual_import",
            document_id: &document_id,
            mime_type,
            original_filename: &original_filename,
            source_path,
        };
        store
            .register_import(&input, restore_deleted_document_id)
            .map_err(|_| RuntimeError::new("import_failed"))
    }

    pub(crate) fn delete_source_document(&self, document_id: &str) -> Result<(), RuntimeError> {
        if document_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let audit_id = random_identifier("audit");
        let result = store.delete_source_document(document_id, &audit_id);
        self.document_passwords()?.remove(document_id);
        result.map_err(|error| {
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.kind() == io::ErrorKind::NotFound)
            {
                RuntimeError::new("document_unavailable")
            } else {
                RuntimeError::new("delete_source_failed")
            }
        })
    }

    pub(crate) fn source_document_copy_context(
        &self,
        document_id: &str,
    ) -> Result<(String, u64), RuntimeError> {
        if document_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let mime_type = store
            .source_document_mime_type(document_id)
            .map_err(|_| RuntimeError::new("document_unavailable"))?;
        let generation = self.inner.vault_session_generation.load(Ordering::SeqCst);
        Ok((mime_type, generation))
    }

    pub(crate) fn save_source_document_copy(
        &self,
        document_id: &str,
        destination: &Path,
        session_generation: u64,
    ) -> Result<(), RuntimeError> {
        if document_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        ensure_copy_outside_vault(&self.inner.root, destination)?;
        self.require_vault_session(session_generation)?;
        // Keep the session mutex through the write so Vault lock cannot report
        // success while the decrypted source buffer is still alive.
        let store = self.store()?;
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != session_generation {
            return Err(RuntimeError::new("vault_locked"));
        }
        let input = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .source_document_input(document_id)
            .map_err(|_| RuntimeError::new("document_unavailable"))?;
        write_export_atomically(destination, &input.plaintext)
            .map_err(|_| RuntimeError::new("source_copy_save_failed"))
    }

    fn require_unlocked(&self) -> Result<(), RuntimeError> {
        if self.store()?.is_none() {
            return Err(RuntimeError::new("vault_locked"));
        }
        Ok(())
    }

    fn require_vault_session(&self, generation: u64) -> Result<(), RuntimeError> {
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != generation {
            return Err(RuntimeError::new("vault_locked"));
        }
        self.require_unlocked()
    }

    fn advance_vault_session(&self) {
        self.inner
            .vault_session_generation
            .fetch_add(1, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(crate) fn save_statement_password(
        &self,
        money_source_id: &str,
        password: &[u8],
    ) -> Result<(), RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        if money_source_id.is_empty() {
            return Err(RuntimeError::new("invalid_source_request"));
        }
        if password.is_empty() {
            return Err(RuntimeError::new("statement_password_required"));
        }
        self.save_statement_password_in_store(store, money_source_id, password)
    }

    fn save_statement_password_in_store(
        &self,
        store: &ManualImportStore,
        money_source_id: &str,
        password: &[u8],
    ) -> Result<(), RuntimeError> {
        self.reconcile_statement_passwords(store)?;
        let state = store
            .statement_password_state(money_source_id)
            .map_err(|_| RuntimeError::new("invalid_source_request"))?;
        match state {
            Some(state) if state.status == StatementPasswordStatus::Saved => {
                self.save_verified_statement_password(&state.secret_storage_key, password)
            }
            None => {
                let secret_storage_key = statement_password_storage_key(money_source_id);
                store
                    .begin_statement_password_save(money_source_id, &secret_storage_key)
                    .map_err(|_| RuntimeError::new("invalid_source_request"))?;
                self.save_verified_statement_password(&secret_storage_key, password)?;
                store
                    .mark_statement_password_saved(money_source_id)
                    .map_err(|_| RuntimeError::new("statement_password_save_failed"))?;
                Ok(())
            }
            Some(_) => Err(RuntimeError::new("statement_password_state_invalid")),
        }
    }

    fn save_verified_statement_password(
        &self,
        secret_ref: &str,
        password: &[u8],
    ) -> Result<(), RuntimeError> {
        self.inner
            .statement_passwords
            .save(secret_ref, password)
            .map_err(|_| RuntimeError::new("statement_password_save_failed"))?;
        let saved = self
            .inner
            .statement_passwords
            .load(secret_ref)
            .map_err(|_| RuntimeError::new("statement_password_save_failed"))?;
        if saved.as_ref().map(|secret| secret.as_slice()) != Some(password) {
            return Err(RuntimeError::new("statement_password_save_failed"));
        }
        Ok(())
    }

    pub(crate) fn remove_statement_password(
        &self,
        money_source_id: &str,
    ) -> Result<(), RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        if money_source_id.is_empty() {
            return Err(RuntimeError::new("invalid_source_request"));
        }
        self.reconcile_statement_passwords(store)?;
        let state = store
            .statement_password_state(money_source_id)
            .map_err(|_| RuntimeError::new("invalid_source_request"))?;
        match state {
            None => Ok(()),
            Some(state) if state.status == StatementPasswordStatus::Saved => {
                store
                    .begin_statement_password_delete(money_source_id)
                    .map_err(|_| RuntimeError::new("statement_password_remove_failed"))?;
                self.inner
                    .statement_passwords
                    .delete(&state.secret_storage_key)
                    .map_err(|_| RuntimeError::new("statement_password_remove_failed"))?;
                store
                    .remove_statement_password_ref(money_source_id, "pending_delete")
                    .map_err(|_| RuntimeError::new("statement_password_remove_failed"))
            }
            Some(_) => Err(RuntimeError::new("statement_password_state_invalid")),
        }
    }

    fn reconcile_statement_passwords(&self, store: &ManualImportStore) -> Result<(), RuntimeError> {
        let states = store
            .pending_statement_password_states()
            .map_err(|_| RuntimeError::new("statement_password_state_invalid"))?;
        for state in states {
            match state.status {
                StatementPasswordStatus::PendingSave => {
                    self.inner
                        .statement_passwords
                        .delete(&state.secret_storage_key)
                        .map_err(|_| RuntimeError::new("statement_password_save_failed"))?;
                    store
                        .remove_statement_password_ref(&state.money_source_id, "pending_save")
                        .map_err(|_| RuntimeError::new("statement_password_save_failed"))?;
                }
                StatementPasswordStatus::PendingDelete => {
                    self.inner
                        .statement_passwords
                        .delete(&state.secret_storage_key)
                        .map_err(|_| RuntimeError::new("statement_password_remove_failed"))?;
                    store
                        .remove_statement_password_ref(&state.money_source_id, "pending_delete")
                        .map_err(|_| RuntimeError::new("statement_password_remove_failed"))?;
                }
                StatementPasswordStatus::Saved => {
                    return Err(RuntimeError::new("statement_password_state_invalid"));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn list_source_documents(
        &self,
        money_source_id: &str,
    ) -> Result<Vec<SourceDocumentSummary>, RuntimeError> {
        if money_source_id.is_empty() {
            return Err(RuntimeError::new("invalid_source_request"));
        }
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let documents = store
            .list_documents(money_source_id)
            .map_err(|_| RuntimeError::new("list_documents_failed"))?;
        documents
            .into_iter()
            .map(|document| self.source_document_summary(store, document))
            .collect()
    }

    pub(crate) fn list_unassigned_source_documents(
        &self,
    ) -> Result<Vec<SourceDocumentSummary>, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let documents = store
            .list_unassigned_documents()
            .map_err(|_| RuntimeError::new("list_documents_failed"))?;
        documents
            .into_iter()
            .map(|document| self.source_document_summary(store, document))
            .collect()
    }

    fn source_document_summary(
        &self,
        store: &ManualImportStore,
        document: SourceDocumentView,
    ) -> Result<SourceDocumentSummary, RuntimeError> {
        let document_status = if document.file_state != "available" {
            "unavailable"
        } else if document.mime_type != "application/pdf" {
            "ready"
        } else {
            match store.source_document_input(&document.document_id) {
                Ok(input) => match pdf_access(&input.plaintext, None) {
                    Ok(PdfAccess::Ready) => "ready",
                    Ok(PdfAccess::PasswordRequired) => {
                        let passwords = self.document_passwords()?;
                        let password = passwords
                            .get(&document.document_id)
                            .map(|value| value.as_slice());
                        match password
                            .and_then(|password| pdf_access(&input.plaintext, Some(password)).ok())
                        {
                            Some(PdfAccess::Ready) => "protected_unlocked",
                            _ => "password_required",
                        }
                    }
                    Err(_) => "inspection_failed",
                },
                Err(_) => "unavailable",
            }
        };
        Ok(SourceDocumentSummary {
            byte_size: document.byte_size,
            document_status,
            document_id: document.document_id,
            file_state: document.file_state,
            mime_type: document.mime_type,
            original_filename: document.original_filename,
            received_at: document.received_at,
        })
    }

    pub(crate) fn statement_password_sources(
        &self,
    ) -> Result<Vec<StatementPasswordSourceSummary>, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .statement_password_sources()
            .map(|sources| {
                sources
                    .into_iter()
                    .map(|source| StatementPasswordSourceSummary {
                        display_name: source.display_name,
                        has_saved_password: source.has_saved_password,
                        money_source_id: source.money_source_id,
                    })
                    .collect()
            })
            .map_err(|_| RuntimeError::new("list_sources_failed"))
    }

    pub(crate) fn try_saved_statement_password(
        &self,
        document_id: &str,
        money_source_id: &str,
    ) -> Result<SavedStatementPasswordResult, RuntimeError> {
        if document_id.is_empty() || money_source_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        ensure_statement_password_source(store, money_source_id)?;
        self.reconcile_statement_passwords(store)?;
        let Some(state) = store
            .statement_password_state(money_source_id)
            .map_err(|_| RuntimeError::new("invalid_source_request"))?
        else {
            return Ok(SavedStatementPasswordResult::Unavailable);
        };
        if state.status != StatementPasswordStatus::Saved {
            return Err(RuntimeError::new("statement_password_state_invalid"));
        }
        let Some(password) = self
            .inner
            .statement_passwords
            .load(&state.secret_storage_key)
            .map_err(|_| RuntimeError::new("statement_password_load_failed"))?
        else {
            return Ok(SavedStatementPasswordResult::Unavailable);
        };
        if !statement_password_unlocks(store, document_id, &password)? {
            return Ok(SavedStatementPasswordResult::Invalid);
        }
        self.document_passwords()?
            .insert(document_id.to_owned(), password);
        Ok(SavedStatementPasswordResult::Unlocked)
    }

    pub(crate) fn unlock_source_document(
        &self,
        document_id: &str,
        money_source_id: &str,
        password: &[u8],
        update_saved_password: bool,
    ) -> Result<(), RuntimeError> {
        if document_id.is_empty() || money_source_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        if password.is_empty() {
            return Err(RuntimeError::new("statement_password_required"));
        }
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        ensure_statement_password_source(store, money_source_id)?;
        if !statement_password_unlocks(store, document_id, password)? {
            return Err(RuntimeError::new("statement_password_invalid"));
        }
        if update_saved_password {
            self.save_statement_password_in_store(store, money_source_id, password)?;
        }
        self.document_passwords()?
            .insert(document_id.to_owned(), Zeroizing::new(password.to_vec()));
        Ok(())
    }

    fn normalization_input(
        &self,
        document_id: &str,
    ) -> Result<SourceDocumentFileInput, RuntimeError> {
        let store = self.store()?;
        let input = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .source_document_input(document_id)
            .map_err(|_| RuntimeError::new("document_unavailable"))?;
        if input.mime_type == "application/pdf" {
            let passwords = self.document_passwords()?;
            let password = passwords.get(document_id).map(|value| value.as_slice());
            match pdf_access(&input.plaintext, None).map_err(document_render_error)? {
                PdfAccess::Ready => {}
                PdfAccess::PasswordRequired => match password {
                    Some(password)
                        if pdf_access(&input.plaintext, Some(password))
                            .map_err(document_render_error)?
                            == PdfAccess::Ready =>
                    {
                        return Err(RuntimeError::new("protected_pdf_normalization_unsupported"));
                    }
                    _ => return Err(RuntimeError::new("statement_password_required")),
                },
            }
        }
        Ok(input)
    }

    fn render_source_document_page(
        &self,
        document_id: &str,
        page_number: u32,
    ) -> Result<RenderedDocumentPage, RuntimeError> {
        if document_id.is_empty() || page_number == 0 {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        // Keep the session mutex through rendering so Vault lock cannot report success
        // while this decrypted page buffer is still alive.
        let store = self.store()?;
        let input = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .source_document_input(document_id)
            .map_err(|_| RuntimeError::new("document_unavailable"))?;
        if input.mime_type != "application/pdf" {
            return Err(RuntimeError::new("viewer_unsupported"));
        }
        let passwords = self.document_passwords()?;
        let password = passwords.get(document_id).map(|value| value.as_slice());
        render_pdf_page_with_password(&input.plaintext, page_number, password)
            .map_err(document_render_error)
    }

    fn preview_source_document(
        &self,
        document_id: &str,
    ) -> Result<SourceDocumentPreview, RuntimeError> {
        if document_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        // Keep the session mutex through preview extraction so Vault lock cannot
        // report success while this decrypted buffer is still alive.
        let store = self.store()?;
        let input = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .source_document_input(document_id)
            .map_err(|_| RuntimeError::new("document_unavailable"))?;
        if input.mime_type != "text/csv" {
            return Err(RuntimeError::new("viewer_unsupported"));
        }
        Ok(bounded_text_preview(&input.plaintext))
    }

    fn apply_normalizer_result(
        &self,
        document_id: &str,
        input_text: &str,
        result: NormalizerResult,
    ) -> Result<SourceDocumentRoutingOutcome, RuntimeError> {
        let proposal = match result {
            NormalizerResult::Classified { proposal } => proposal,
            NormalizerResult::NeedsAttention { reason } => {
                let reason = if reason == "unsupported_document" {
                    "unsupported_document"
                } else {
                    "classification_uncertain"
                };
                return Ok(SourceDocumentRoutingOutcome::needs_attention(
                    document_id,
                    reason,
                ));
            }
        };
        let Some(statement_id) = proposal.document.statement_id.as_deref() else {
            return Ok(SourceDocumentRoutingOutcome::needs_attention(
                document_id,
                "classification_uncertain",
            ));
        };
        if !valid_synthetic_fingerprint(input_text, &proposal) {
            return Ok(SourceDocumentRoutingOutcome::needs_attention(
                document_id,
                "provider_fingerprint_mismatch",
            ));
        }
        let semantic_document_key = format!(
            "{}:{}",
            proposal.document.provider_key.as_str(),
            statement_id
        );
        let account_ids = (0..proposal.accounts.len())
            .map(|_| random_identifier("account"))
            .collect::<Vec<_>>();
        let accounts = proposal
            .accounts
            .iter()
            .zip(&account_ids)
            .map(|(account, account_id)| TrustedAccountCandidate {
                account_id,
                account_type: &account.account_type,
                currency: account.currency.as_deref(),
                display_name: "Detected account",
                masked_identifier: account.masked_identifier.as_deref(),
                provider_account_id: account.provider_account_id.as_deref(),
            })
            .collect::<Vec<_>>();
        let audit_id = random_identifier("audit");
        let classification = TrustedDocumentClassification {
            accounts: &accounts,
            audit_id: &audit_id,
            document_id,
            provider_key: &proposal.document.provider_key,
            semantic_document_key: &semantic_document_key,
        };
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .apply_trusted_classification(&classification)
            .map_err(|_| RuntimeError::new("classification_failed"))
    }

    #[cfg(test)]
    fn seed_money_source(
        &self,
        id: &str,
        provider_key: &str,
        display_name: &str,
        source_type: &str,
    ) -> Result<(), RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .seed_money_source(id, provider_key, display_name, source_type)
            .map_err(|_| RuntimeError::new("seed_failed"))
    }

    fn store(&self) -> Result<MutexGuard<'_, Option<ManualImportStore>>, RuntimeError> {
        let system_lock_generation = self.inner.system_lock_generation.load(Ordering::SeqCst);
        self.store_for_system_generation(system_lock_generation)
    }

    fn store_for_system_generation(
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

    fn raw_store(&self) -> Result<MutexGuard<'_, Option<ManualImportStore>>, RuntimeError> {
        self.inner
            .store
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))
    }

    fn document_passwords(&self) -> Result<MutexGuard<'_, DocumentPasswordSessions>, RuntimeError> {
        self.inner
            .document_passwords
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))
    }

    fn system_session_active(&self) -> bool {
        self.inner.system_session_active.load(Ordering::SeqCst)
    }

    fn load_remembered_master_key(&self) -> Result<Option<Zeroizing<[u8; KEY_LEN]>>, ()> {
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

fn document_render_error(error: io::Error) -> RuntimeError {
    match error.kind() {
        io::ErrorKind::InvalidInput => RuntimeError::new("invalid_document_request"),
        io::ErrorKind::PermissionDenied => RuntimeError::new("statement_password_required"),
        io::ErrorKind::Unsupported => RuntimeError::new("viewer_unsupported"),
        _ => RuntimeError::new("document_render_failed"),
    }
}

fn bounded_text_preview(plaintext: &[u8]) -> SourceDocumentPreview {
    // from_utf8_lossy copies the whole buffer for invalid UTF-8; keep every
    // full-plaintext copy inside a zeroizing wrapper like the normalizer path.
    let text = Zeroizing::new(String::from_utf8_lossy(plaintext).into_owned());
    let mut line_count = 0_usize;
    let mut preview_lines = 0_usize;
    let mut preview_text = String::new();
    let mut truncated = false;
    for line in text.lines() {
        line_count += 1;
        if truncated || preview_lines == PREVIEW_MAX_LINES {
            truncated = true;
            continue;
        }
        let separator = usize::from(!preview_text.is_empty());
        if preview_text.len() + separator + line.len() <= PREVIEW_MAX_BYTES {
            if separator == 1 {
                preview_text.push('\n');
            }
            preview_text.push_str(line);
            preview_lines += 1;
            continue;
        }
        // Keep a cut prefix of an overlong line so a single huge row still
        // previews; stop at a UTF-8 boundary.
        let remaining = PREVIEW_MAX_BYTES.saturating_sub(preview_text.len() + separator);
        let cut = line.floor_char_boundary(remaining);
        if cut > 0 {
            if separator == 1 {
                preview_text.push('\n');
            }
            preview_text.push_str(&line[..cut]);
            preview_lines += 1;
        }
        truncated = true;
    }
    SourceDocumentPreview {
        line_count: line_count as u64,
        preview_lines: preview_lines as u64,
        preview_text,
        truncated,
    }
}

fn statement_password_unlocks(
    store: &ManualImportStore,
    document_id: &str,
    password: &[u8],
) -> Result<bool, RuntimeError> {
    let input = store
        .source_document_input(document_id)
        .map_err(|_| RuntimeError::new("document_unavailable"))?;
    if input.mime_type != "application/pdf" {
        return Err(RuntimeError::new("viewer_unsupported"));
    }
    match pdf_access(&input.plaintext, None).map_err(document_render_error)? {
        PdfAccess::Ready => Err(RuntimeError::new("document_not_protected")),
        PdfAccess::PasswordRequired => pdf_access(&input.plaintext, Some(password))
            .map(|access| access == PdfAccess::Ready)
            .map_err(document_render_error),
    }
}

fn ensure_statement_password_source(
    store: &ManualImportStore,
    money_source_id: &str,
) -> Result<(), RuntimeError> {
    if store
        .statement_password_sources()
        .map_err(|_| RuntimeError::new("invalid_source_request"))?
        .iter()
        .any(|source| source.money_source_id == money_source_id)
    {
        Ok(())
    } else {
        Err(RuntimeError::new("invalid_source_request"))
    }
}

#[tauri::command]
pub(crate) async fn vault_status(
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.status())
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn vault_access_status(
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultAccessStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.access_status())
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn create_vault(
    password: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let password = Zeroizing::new(password);
    tauri::async_runtime::spawn_blocking(move || runtime.create(password.as_bytes()))
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn unlock_vault(
    password: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let password = Zeroizing::new(password);
    tauri::async_runtime::spawn_blocking(move || runtime.unlock(password.as_bytes()))
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn unlock_vault_with_keychain(
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.unlock_with_keychain())
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn remember_vault_on_this_mac(
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.remember_on_this_mac())
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn forget_vault_on_this_mac(
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.forget_this_mac())
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn remove_statement_password(
    money_source_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        runtime.remove_statement_password(&money_source_id)
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn list_statement_password_sources(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<StatementPasswordSourceSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.statement_password_sources())
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn try_saved_statement_password(
    document_id: String,
    money_source_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<SavedStatementPasswordResult, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        runtime.try_saved_statement_password(&document_id, &money_source_id)
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn unlock_source_document(
    document_id: String,
    money_source_id: String,
    password: String,
    update_saved_password: bool,
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    let runtime = runtime.inner().clone();
    let password = Zeroizing::new(password);
    tauri::async_runtime::spawn_blocking(move || {
        runtime.unlock_source_document(
            &document_id,
            &money_source_id,
            password.as_bytes(),
            update_saved_password,
        )
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn lock_vault(
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.lock())
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn save_recovery_file(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<bool, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, RuntimeError> {
        runtime.require_unlocked()?;
        let selected = app
            .dialog()
            .file()
            .set_title("Save your CanCan recovery file")
            .set_file_name("CanCan Recovery.cancan-recovery")
            .add_filter("CanCan recovery file", &["cancan-recovery"])
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(false);
        };
        let destination = selected
            .into_path()
            .map_err(|_| RuntimeError::new("file_selection_failed"))?;
        runtime.save_recovery_file(&destination)?;
        Ok(true)
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn save_source_document_copy(
    document_id: String,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<bool, VaultCommandError> {
    if document_id.is_empty() {
        return Err(VaultCommandError::new("invalid_document_request"));
    }
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, RuntimeError> {
        let (mime_type, session_generation) =
            runtime.source_document_copy_context(&document_id)?;
        let confirmed = app
            .dialog()
            .message(
                "Save a normal file outside CanCan's encrypted Vault? The saved copy will no longer be protected by CanCan and becomes your responsibility.",
            )
            .title("Save a copy outside the Vault?")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Save a copy".to_owned(),
                "Cancel".to_owned(),
            ))
            .blocking_show();
        if !confirmed {
            return Ok(false);
        }
        runtime.require_vault_session(session_generation)?;
        let default_name = match mime_type.as_str() {
            "application/pdf" => "CanCan source copy.pdf",
            "text/csv" => "CanCan source copy.csv",
            _ => return Err(RuntimeError::new("unsupported_document")),
        };
        let selected = app
            .dialog()
            .file()
            .set_title("Save a copy")
            .set_file_name(default_name)
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(false);
        };
        let destination = selected
            .into_path()
            .map_err(|_| RuntimeError::new("file_selection_failed"))?;
        runtime.save_source_document_copy(&document_id, &destination, session_generation)?;
        Ok(true)
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn import_source_document(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<Option<SourceDocumentImportOutcome>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(
        move || -> Result<Option<SourceDocumentImportOutcome>, RuntimeError> {
        runtime.require_unlocked()?;
        let selected = app
            .dialog()
            .file()
            .set_title("Import a statement")
            .add_filter("Financial documents", &["pdf", "csv"])
            .blocking_pick_file();
        let Some(selected) = selected else {
            return Ok(None);
        };
        let path = selected
            .into_path()
            .map_err(|_| RuntimeError::new("file_selection_failed"))?;
        let outcome = runtime.import_selected_document(&path, None)?;
        if outcome.status != SourceDocumentImportStatus::RestoreConfirmationRequired {
            return Ok(Some(outcome));
        }
        let restore = app
            .dialog()
            .message(
                "This exact file was previously deleted from CanCan's Vault. Restore it to the existing document entry?",
            )
            .title("Restore source file?")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Restore file".to_owned(),
                "Cancel".to_owned(),
            ))
            .blocking_show();
        if !restore {
            return Ok(None);
        }
        runtime
            .import_selected_document(&path, Some(&outcome.document_id))
            .map(Some)
        },
    )
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn delete_source_document(
    document_id: String,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<bool, VaultCommandError> {
    if document_id.is_empty() {
        return Err(VaultCommandError::new("invalid_document_request"));
    }
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<bool, RuntimeError> {
        runtime.require_unlocked()?;
        let confirmed = app
            .dialog()
            .message(
                "Delete the encrypted source file stored in CanCan's Vault? The document entry, record history, audit trail, and ledger links will remain and show Source file deleted. New backups will not include this file; older backups or copies saved outside CanCan may still contain it.",
            )
            .title("Delete source file?")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Delete source file".to_owned(),
                "Cancel".to_owned(),
            ))
            .blocking_show();
        if !confirmed {
            return Ok(false);
        }
        runtime.delete_source_document(&document_id)?;
        Ok(true)
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn list_unassigned_source_documents(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<SourceDocumentSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.list_unassigned_source_documents())
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn normalize_source_document(
    document_id: String,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<SourceDocumentRoutingOutcome, VaultCommandError> {
    if document_id.is_empty() {
        return Err(VaultCommandError::new("invalid_document_request"));
    }
    let runtime = runtime.inner().clone();
    let input = {
        let runtime = runtime.clone();
        let document_id = document_id.clone();
        tauri::async_runtime::spawn_blocking(move || runtime.normalization_input(&document_id))
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    let input_text = Zeroizing::new(String::from_utf8_lossy(&input.plaintext).into_owned());
    let result = run_normalizer_sidecar(&app, &document_id, &input.mime_type, &input_text)
        .await
        .map_err(VaultCommandError::from)?;
    tauri::async_runtime::spawn_blocking(move || {
        runtime.apply_normalizer_result(&document_id, &input_text, result)
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn list_source_documents(
    money_source_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<SourceDocumentSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.list_source_documents(&money_source_id))
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn render_source_document_page(
    document_id: String,
    page_number: u32,
    runtime: State<'_, VaultRuntime>,
) -> Result<RenderedDocumentPage, VaultCommandError> {
    if document_id.is_empty() || page_number == 0 {
        return Err(VaultCommandError::new("invalid_document_request"));
    }
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        runtime.render_source_document_page(&document_id, page_number)
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn preview_source_document(
    document_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<SourceDocumentPreview, VaultCommandError> {
    if document_id.is_empty() {
        return Err(VaultCommandError::new("invalid_document_request"));
    }
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.preview_source_document(&document_id))
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(Into::into)
}

fn source_document_metadata(source_path: &Path) -> Result<(String, &'static str), RuntimeError> {
    let mime_type = match source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("pdf") => "application/pdf",
        Some("csv") => "text/csv",
        _ => return Err(RuntimeError::new("unsupported_document")),
    };
    let metadata = fs::metadata(source_path).map_err(|_| RuntimeError::new("import_failed"))?;
    if !metadata.is_file() {
        return Err(RuntimeError::new("unsupported_document"));
    }
    let original_filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| RuntimeError::new("unsupported_document"))?
        .to_owned();
    Ok((original_filename, mime_type))
}

async fn run_normalizer_sidecar(
    app: &AppHandle,
    document_id: &str,
    mime_type: &str,
    content: &str,
) -> Result<NormalizerResult, RuntimeError> {
    let request_id = random_identifier("normalize");
    let command = Zeroizing::new(
        serde_json::to_vec(&NormalizerCommand {
            content,
            document_id,
            mime_type,
            request_id: &request_id,
            kind: "normalize",
        })
        .map_err(|_| RuntimeError::new("normalizer_unavailable"))?,
    );
    let sidecar = app
        .shell()
        .sidecar("cancan-document-normalizer")
        .map_err(|_| RuntimeError::new("normalizer_unavailable"))?
        .env_clear();
    let (mut events, mut child) = sidecar
        .spawn()
        .map_err(|_| RuntimeError::new("normalizer_unavailable"))?;
    let deadline = Instant::now() + NORMALIZER_TIMEOUT;
    let mut ready = false;
    let result = loop {
        let event = match timeout_at(deadline, events.recv()).await {
            Ok(Some(event)) => event,
            Ok(None) | Err(_) => return fail_normalizer(child),
        };
        match event {
            CommandEvent::Stdout(bytes) => {
                let message = match serde_json::from_slice::<NormalizerMessage>(&bytes) {
                    Ok(message) => message,
                    Err(_) => return fail_normalizer(child),
                };
                match message {
                    NormalizerMessage::Ready {
                        protocol_version,
                        runtime,
                        environment_cleared,
                    } if !ready
                        && valid_normalizer_ready(
                            protocol_version,
                            &runtime,
                            environment_cleared,
                        ) =>
                    {
                        let mut framed = Zeroizing::new(command.to_vec());
                        framed.push(b'\n');
                        if child.write(&framed).is_err() {
                            return fail_normalizer(child);
                        }
                        ready = true;
                    }
                    NormalizerMessage::Result {
                        request_id: response_id,
                        result,
                    } if ready && response_id == request_id => break result,
                    NormalizerMessage::Ready { .. }
                    | NormalizerMessage::Result { .. }
                    | NormalizerMessage::Error => return fail_normalizer(child),
                }
            }
            CommandEvent::Terminated(_) => {
                return Err(RuntimeError::new("normalizer_failed"));
            }
            CommandEvent::Stderr(_) | CommandEvent::Error(_) => return fail_normalizer(child),
            _ => return fail_normalizer(child),
        }
    };

    if child.write(b"{\"type\":\"shutdown\"}\n").is_err() {
        return fail_normalizer(child);
    }
    let shutdown_deadline = Instant::now() + NORMALIZER_SHUTDOWN_TIMEOUT;
    let event = match timeout_at(shutdown_deadline, events.recv()).await {
        Ok(Some(event)) => event,
        Ok(None) | Err(_) => return fail_normalizer(child),
    };
    match event {
        CommandEvent::Terminated(payload) if payload.code == Some(0) => Ok(result),
        CommandEvent::Terminated(_) => Err(RuntimeError::new("normalizer_failed")),
        CommandEvent::Stdout(_) | CommandEvent::Stderr(_) | CommandEvent::Error(_) => {
            fail_normalizer(child)
        }
        _ => fail_normalizer(child),
    }
}

fn fail_normalizer(child: CommandChild) -> Result<NormalizerResult, RuntimeError> {
    let _ = child.kill();
    Err(RuntimeError::new("normalizer_failed"))
}

fn valid_normalizer_ready(protocol_version: u8, runtime: &str, environment_cleared: bool) -> bool {
    protocol_version == 1 && runtime == "single-pass-mock" && environment_cleared
}

fn valid_synthetic_fingerprint(content: &str, proposal: &NormalizerProposal) -> bool {
    content.contains("CANCAN_SYNTHETIC_STATEMENT_V1")
        && content.contains("provider=synthetic-bank")
        && content.contains("statement_id=transfer-2026-07")
        && proposal.document.provider_key == "synthetic-bank"
        && proposal.document.document_type == "transfer_export"
        && proposal.document.statement_id.as_deref() == Some("transfer-2026-07")
        && !proposal.accounts.is_empty()
        && proposal.accounts.iter().all(|account| {
            matches!(
                account.account_type.as_str(),
                "deposit_account"
                    | "credit_card"
                    | "currency_balance"
                    | "brokerage_account"
                    | "cash_balance"
                    | "position_group"
                    | "insurance_policy"
                    | "manual_asset"
                    | "manual_liability"
            )
        })
}

fn random_identifier(prefix: &str) -> String {
    let mut random = [0_u8; 16];
    OsRng.fill_bytes(&mut random);
    let mut identifier = String::with_capacity(prefix.len() + 1 + random.len() * 2);
    identifier.push_str(prefix);
    identifier.push('-');
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut identifier, "{byte:02x}").expect("writing to String cannot fail");
    }
    identifier
}

fn statement_password_storage_key(money_source_id: &str) -> String {
    format!("money-source:{money_source_id}")
}

fn candidate_name() -> String {
    let mut random = [0_u8; 8];
    OsRng.fill_bytes(&mut random);
    let mut name = String::from(".vault-create-");
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut name, "{byte:02x}").expect("writing to String cannot fail");
    }
    name
}

fn write_new_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent directory"))?;
    let temporary = parent.join(format!(
        ".cancan-write-{}.tmp",
        random_identifier("recovery")
    ));
    let result = (|| {
        write_new_synced(&temporary, bytes)?;
        fs::rename(&temporary, path)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn ensure_copy_outside_vault(vault_root: &Path, destination: &Path) -> Result<(), RuntimeError> {
    let vault_root = fs::canonicalize(vault_root)
        .map_err(|_| RuntimeError::new("source_copy_location_invalid"))?;
    let parent = destination
        .parent()
        .ok_or_else(|| RuntimeError::new("source_copy_location_invalid"))?;
    let parent =
        fs::canonicalize(parent).map_err(|_| RuntimeError::new("source_copy_location_invalid"))?;
    if parent.starts_with(&vault_root)
        || destination
            .canonicalize()
            .is_ok_and(|path| path.starts_with(&vault_root))
    {
        return Err(RuntimeError::new("source_copy_location_invalid"));
    }
    Ok(())
}

fn write_export_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent directory"))?;
    let temporary = parent.join(format!(".cancan-export-{}.tmp", random_identifier("copy")));
    let result = (|| {
        write_new_synced(&temporary, bytes)?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    } else {
        // After rename, the complete user copy is visible and cannot be rolled back safely
        // without risking removal of a valid export.
        let _ = sync_directory(parent);
    }
    result
}

fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::SourceDocumentImportStatus;
    use crate::vault::open_recovery_file;
    #[cfg(target_os = "macos")]
    use crate::viewer::tests::protected_pdf_fixture;
    use std::{collections::HashMap, thread, time::Duration};

    #[derive(Default)]
    struct MemoryRememberedKeyStore {
        secret: Mutex<Option<Vec<u8>>>,
    }

    impl RememberedKeyStore for MemoryRememberedKeyStore {
        fn delete(&self) -> Result<(), ()> {
            *self.secret.lock().map_err(|_| ())? = None;
            Ok(())
        }

        fn is_present(&self) -> Result<bool, ()> {
            Ok(self.secret.lock().map_err(|_| ())?.is_some())
        }

        fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
            Ok(self
                .secret
                .lock()
                .map_err(|_| ())?
                .clone()
                .map(Zeroizing::new))
        }

        fn save(&self, secret: &[u8]) -> Result<(), ()> {
            *self.secret.lock().map_err(|_| ())? = Some(secret.to_vec());
            Ok(())
        }
    }

    struct FailingRememberedKeyStore;

    impl RememberedKeyStore for FailingRememberedKeyStore {
        fn delete(&self) -> Result<(), ()> {
            Err(())
        }

        fn is_present(&self) -> Result<bool, ()> {
            Err(())
        }

        fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
            Ok(None)
        }

        fn save(&self, _secret: &[u8]) -> Result<(), ()> {
            Err(())
        }
    }

    struct PresenceOnlyRememberedKeyStore;

    impl RememberedKeyStore for PresenceOnlyRememberedKeyStore {
        fn delete(&self) -> Result<(), ()> {
            Err(())
        }

        fn is_present(&self) -> Result<bool, ()> {
            Ok(true)
        }

        fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
            panic!("status must not load the remembered secret")
        }

        fn save(&self, _secret: &[u8]) -> Result<(), ()> {
            Err(())
        }
    }

    struct MalformedDeleteFailingRememberedKeyStore;

    impl RememberedKeyStore for MalformedDeleteFailingRememberedKeyStore {
        fn delete(&self) -> Result<(), ()> {
            Err(())
        }

        fn is_present(&self) -> Result<bool, ()> {
            Ok(true)
        }

        fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
            Ok(Some(Zeroizing::new(vec![0x55; KEY_LEN - 1])))
        }

        fn save(&self, _secret: &[u8]) -> Result<(), ()> {
            Err(())
        }
    }

    #[derive(Default)]
    struct MemoryStatementPasswordStore {
        fail_delete: AtomicBool,
        fail_save: AtomicBool,
        secrets: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl StatementPasswordStore for MemoryStatementPasswordStore {
        fn delete(&self, secret_ref: &str) -> Result<(), ()> {
            if self.fail_delete.load(Ordering::SeqCst) {
                return Err(());
            }
            self.secrets.lock().map_err(|_| ())?.remove(secret_ref);
            Ok(())
        }

        fn load(&self, secret_ref: &str) -> Result<Option<Zeroizing<Vec<u8>>>, ()> {
            Ok(self
                .secrets
                .lock()
                .map_err(|_| ())?
                .get(secret_ref)
                .cloned()
                .map(Zeroizing::new))
        }

        fn save(&self, secret_ref: &str, secret: &[u8]) -> Result<(), ()> {
            if self.fail_save.load(Ordering::SeqCst) {
                return Err(());
            }
            self.secrets
                .lock()
                .map_err(|_| ())?
                .insert(secret_ref.to_owned(), secret.to_vec());
            Ok(())
        }
    }

    fn statement_password_runtime(
        root: &Path,
        statement_passwords: Arc<dyn StatementPasswordStore>,
    ) -> VaultRuntime {
        let runtime = VaultRuntime::with_secret_stores(
            root.to_path_buf(),
            Arc::new(MemoryRememberedKeyStore::default()),
            statement_passwords,
        );
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        runtime
            .seed_money_source("source-dbs", "dbs", "DBS", "bank")
            .expect("seed Money Source");
        runtime
    }

    fn statement_password_state(
        runtime: &VaultRuntime,
    ) -> Option<crate::database::StatementPasswordState> {
        runtime
            .store()
            .expect("active store")
            .as_ref()
            .expect("unlocked store")
            .statement_password_state("source-dbs")
            .expect("statement password state")
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn protects_pdf_passwords_inside_the_unlocked_vault_session() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
        let runtime =
            statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());
        let source = parent.path().join("protected-statement.pdf");
        let mut protected_fixture = protected_pdf_fixture();
        protected_fixture.extend_from_slice(
            b"\nCANCAN_SYNTHETIC_STATEMENT_V1\nprovider=synthetic-bank\nstatement_id=transfer-2026-07",
        );
        fs::write(&source, protected_fixture).expect("write protected PDF fixture");
        let outcome = runtime
            .import_selected_document(&source, None)
            .expect("import protected statement");

        let documents = runtime
            .list_unassigned_source_documents()
            .expect("list protected statement");
        assert_eq!(documents[0].document_id, outcome.document_id);
        assert_eq!(documents[0].document_status, "password_required");
        assert_eq!(
            runtime
                .normalization_input(&outcome.document_id)
                .err()
                .expect("protected PDF must not normalize before unlock")
                .code(),
            "statement_password_required"
        );
        assert_eq!(
            runtime
                .render_source_document_page(&outcome.document_id, 1)
                .expect_err("locked PDF must not render")
                .code(),
            "statement_password_required"
        );
        assert_eq!(
            runtime
                .unlock_source_document(
                    &outcome.document_id,
                    "source-dbs",
                    b"wrong-password",
                    true,
                )
                .expect_err("wrong password must not save")
                .code(),
            "statement_password_invalid"
        );
        assert_eq!(statement_password_state(&runtime), None);

        runtime
            .unlock_source_document(
                &outcome.document_id,
                "source-dbs",
                b"statement-password",
                false,
            )
            .expect("use password once");
        assert_eq!(
            runtime
                .list_unassigned_source_documents()
                .expect("list session-unlocked statement")[0]
                .document_status,
            "protected_unlocked"
        );
        assert_eq!(
            runtime
                .normalization_input(&outcome.document_id)
                .err()
                .expect("protected PDF extraction is not implemented")
                .code(),
            "protected_pdf_normalization_unsupported"
        );
        runtime
            .render_source_document_page(&outcome.document_id, 1)
            .expect("render session-unlocked statement");
        runtime.request_system_lock().expect("system-lock Vault");
        runtime
            .resume_system_session()
            .expect("resume system session");
        runtime
            .unlock(b"synthetic-vault-password")
            .expect("reopen system-locked Vault");
        assert_eq!(
            runtime
                .list_unassigned_source_documents()
                .expect("list after session lock")[0]
                .document_status,
            "password_required"
        );

        runtime
            .unlock_source_document(
                &outcome.document_id,
                "source-dbs",
                b"statement-password",
                true,
            )
            .expect("verify and save password");
        assert_eq!(
            statement_passwords
                .load("money-source:source-dbs")
                .expect("load saved password")
                .expect("saved password exists")
                .as_slice(),
            b"statement-password"
        );
        runtime.lock().expect("lock saved-password session");
        runtime
            .unlock(b"synthetic-vault-password")
            .expect("reopen saved-password Vault");
        assert_eq!(
            runtime
                .try_saved_statement_password(&outcome.document_id, "source-dbs")
                .expect("try saved password"),
            SavedStatementPasswordResult::Unlocked
        );
        assert_eq!(
            runtime
                .list_unassigned_source_documents()
                .expect("list saved-password statement")[0]
                .document_status,
            "protected_unlocked"
        );

        statement_passwords
            .delete("money-source:source-dbs")
            .expect("remove device-local saved password");
        assert_eq!(
            runtime
                .try_saved_statement_password(&outcome.document_id, "source-dbs")
                .expect("report unavailable saved password"),
            SavedStatementPasswordResult::Unavailable
        );
        assert_eq!(
            statement_password_state(&runtime)
                .expect("preserve saved-password reference")
                .status,
            StatementPasswordStatus::Saved
        );
        statement_passwords
            .save("money-source:source-dbs", b"wrong-password")
            .expect("save invalid password fixture");
        assert_eq!(
            runtime
                .try_saved_statement_password(&outcome.document_id, "source-dbs")
                .expect("report invalid saved password"),
            SavedStatementPasswordResult::Invalid
        );

        let protected_copy = parent.path().join("protected-copy.pdf");
        let (_, copy_generation) = runtime
            .source_document_copy_context(&outcome.document_id)
            .expect("read protected copy context");
        runtime
            .save_source_document_copy(&outcome.document_id, &protected_copy, copy_generation)
            .expect("save protected source copy");
        let copied_bytes = fs::read(&protected_copy).expect("read protected source copy");
        assert_eq!(
            copied_bytes,
            fs::read(&source).expect("read original protected source")
        );
        assert_eq!(
            pdf_access(&copied_bytes, None).expect("inspect protected source copy"),
            PdfAccess::PasswordRequired
        );

        let files_directory = parent.path().join("vault").join("files");
        let original_permissions = fs::metadata(&files_directory)
            .expect("read files directory metadata")
            .permissions();
        fs::set_permissions(&files_directory, fs::Permissions::from_mode(0o500))
            .expect("make encrypted files directory read-only");
        let deletion = runtime.delete_source_document(&outcome.document_id);
        fs::set_permissions(&files_directory, original_permissions)
            .expect("restore files directory permissions");
        assert_eq!(
            deletion
                .expect_err("surface encrypted-blob cleanup failure")
                .code(),
            "delete_source_failed"
        );
        assert!(
            !runtime
                .document_passwords()
                .expect("document password cache")
                .contains_key(&outcome.document_id),
            "deletion must clear the document password even when blob cleanup fails"
        );
        assert_eq!(
            runtime
                .source_document_copy_context(&outcome.document_id)
                .expect_err("deleted source must not open an export dialog")
                .code(),
            "document_unavailable"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn fails_closed_when_a_pdf_cannot_be_inspected() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        let source = parent.path().join("corrupt.pdf");
        fs::write(
            &source,
            b"%PDF corrupt\nCANCAN_SYNTHETIC_STATEMENT_V1\nprovider=synthetic-bank\nstatement_id=transfer-2026-07",
        )
        .expect("write corrupt PDF fixture");
        let outcome = runtime
            .import_selected_document(&source, None)
            .expect("import corrupt PDF");

        assert_eq!(
            runtime
                .list_unassigned_source_documents()
                .expect("list corrupt PDF")[0]
                .document_status,
            "inspection_failed"
        );
        assert_eq!(
            runtime
                .normalization_input(&outcome.document_id)
                .err()
                .expect("corrupt PDF must not reach normalization")
                .code(),
            "document_render_failed"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn keeps_listing_other_documents_when_one_encrypted_blob_is_unreadable() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let vault_root = parent.path().join("vault");
        let runtime = VaultRuntime::new(vault_root.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");

        let pdf = parent.path().join("protected-statement.pdf");
        fs::write(&pdf, protected_pdf_fixture()).expect("write protected PDF fixture");
        let pdf_outcome = runtime
            .import_selected_document(&pdf, None)
            .expect("import protected statement");
        let csv = parent.path().join("transactions.csv");
        fs::write(&csv, b"date,amount\n2026-07-01,10.00\n").expect("write CSV fixture");
        let csv_outcome = runtime
            .import_selected_document(&csv, None)
            .expect("import CSV");

        let encrypted_locator = {
            let store = runtime.store().expect("active store");
            store
                .as_ref()
                .expect("unlocked store")
                .list_unassigned_documents()
                .expect("list imported documents")
                .into_iter()
                .find(|document| document.document_id == pdf_outcome.document_id)
                .and_then(|document| document.encrypted_locator)
                .expect("protected PDF encrypted locator")
        };
        fs::remove_file(vault_root.join(encrypted_locator))
            .expect("remove protected PDF encrypted blob");

        let documents = runtime
            .list_unassigned_source_documents()
            .expect("list remaining documents");
        assert_eq!(documents.len(), 2);
        assert_eq!(
            documents
                .iter()
                .find(|document| document.document_id == pdf_outcome.document_id)
                .expect("unreadable PDF row")
                .document_status,
            "unavailable"
        );
        assert_eq!(
            documents
                .iter()
                .find(|document| document.document_id == csv_outcome.document_id)
                .expect("readable CSV row")
                .document_status,
            "ready"
        );
    }

    #[test]
    fn accepts_only_the_expected_normalizer_handshake() {
        assert!(valid_normalizer_ready(1, "single-pass-mock", true));
        assert!(!valid_normalizer_ready(2, "single-pass-mock", true));
        assert!(!valid_normalizer_ready(1, "live-runtime", true));
        assert!(!valid_normalizer_ready(1, "single-pass-mock", false));
    }

    #[test]
    fn maps_platform_and_document_render_failures_separately() {
        assert_eq!(
            document_render_error(io::Error::from(io::ErrorKind::Unsupported)).code(),
            "viewer_unsupported"
        );
        assert_eq!(
            document_render_error(io::Error::from(io::ErrorKind::InvalidInput)).code(),
            "invalid_document_request"
        );
        assert_eq!(
            document_render_error(io::Error::from(io::ErrorKind::InvalidData)).code(),
            "document_render_failed"
        );
    }

    #[test]
    fn saves_an_atomic_plaintext_copy_only_outside_the_vault() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let source = parent.path().join("statement.csv");
        let source_bytes = b"date,amount\n2026-07-22,42";
        fs::write(&source, source_bytes).expect("write source fixture");
        let root = parent.path().join("vault");
        let runtime = VaultRuntime::new(root.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        let imported = runtime
            .import_selected_document(&source, None)
            .expect("import source");

        let (mime_type, session_generation) = runtime
            .source_document_copy_context(&imported.document_id)
            .expect("read source copy context");
        assert_eq!(mime_type, "text/csv");
        let destination = parent.path().join("statement-copy.csv");
        fs::write(&destination, b"existing copy").expect("write existing destination");
        runtime
            .save_source_document_copy(&imported.document_id, &destination, session_generation)
            .expect("save source copy");
        assert_eq!(
            fs::read(&destination).expect("read saved copy"),
            source_bytes
        );
        assert!(
            fs::read_dir(parent.path())
                .expect("list export directory")
                .all(|entry| !entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".cancan-export-"))
        );

        assert_eq!(
            runtime
                .save_source_document_copy(
                    &imported.document_id,
                    &root.join("copy-inside-vault.csv"),
                    session_generation,
                )
                .expect_err("reject copy inside Vault")
                .code(),
            "source_copy_location_invalid"
        );
        runtime.lock().expect("lock Vault");
        assert_eq!(
            runtime
                .save_source_document_copy(&imported.document_id, &destination, session_generation,)
                .expect_err("reject copy while locked")
                .code(),
            "vault_locked"
        );
    }

    #[cfg(unix)]
    #[test]
    fn preserves_the_existing_destination_when_source_copy_write_fails() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let source = parent.path().join("statement.csv");
        fs::write(&source, b"date,amount\n2026-07-22,42").expect("write source fixture");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        let imported = runtime
            .import_selected_document(&source, None)
            .expect("import source");
        let destination_directory = parent.path().join("exports");
        fs::create_dir(&destination_directory).expect("create export directory");
        let destination = destination_directory.join("statement.csv");
        fs::write(&destination, b"existing copy").expect("write existing destination");
        let original_permissions = fs::metadata(&destination_directory)
            .expect("read destination permissions")
            .permissions();
        fs::set_permissions(&destination_directory, fs::Permissions::from_mode(0o500))
            .expect("make destination read-only");

        let (_, session_generation) = runtime
            .source_document_copy_context(&imported.document_id)
            .expect("read source copy context");
        let result = runtime.save_source_document_copy(
            &imported.document_id,
            &destination,
            session_generation,
        );
        fs::set_permissions(&destination_directory, original_permissions)
            .expect("restore destination permissions");

        assert_eq!(
            result.expect_err("surface copy write failure").code(),
            "source_copy_save_failed"
        );
        assert_eq!(
            fs::read(&destination).expect("read preserved destination"),
            b"existing copy"
        );
        assert!(
            fs::read_dir(&destination_directory)
                .expect("list destination directory")
                .all(|entry| !entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".cancan-export-"))
        );

        let directory_destination = destination_directory.join("existing-directory");
        fs::create_dir(&directory_destination).expect("create directory destination");
        assert_eq!(
            runtime
                .save_source_document_copy(
                    &imported.document_id,
                    &directory_destination,
                    session_generation,
                )
                .expect_err("rename over a directory must fail")
                .code(),
            "source_copy_save_failed"
        );
        assert!(directory_destination.is_dir());
        assert!(
            fs::read_dir(&destination_directory)
                .expect("list destination after rename failure")
                .all(|entry| !entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".cancan-export-"))
        );
    }

    #[test]
    fn creates_locks_and_unlocks_a_vault_without_exposing_the_master_key() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let runtime = VaultRuntime::new(parent.path().join("vault"));

        assert_eq!(
            runtime.status().expect("initial status"),
            VaultStatus::NotCreated
        );
        assert_eq!(
            runtime
                .create(b"synthetic-vault-password")
                .expect("create Vault"),
            VaultStatus::Unlocked
        );
        let wrapper = fs::read(parent.path().join("vault").join(KEY_FILE_NAME))
            .expect("read password wrapper");
        assert!(
            !wrapper
                .windows("synthetic-vault-password".len())
                .any(|bytes| bytes == b"synthetic-vault-password")
        );
        assert_eq!(runtime.lock().expect("lock Vault"), VaultStatus::Locked);
        assert_eq!(
            runtime
                .unlock(b"wrong-password")
                .expect_err("reject wrong password")
                .code(),
            "invalid_credentials"
        );
        assert_eq!(
            runtime.status().expect("locked status"),
            VaultStatus::Locked
        );
        assert_eq!(
            runtime
                .unlock(b"synthetic-vault-password")
                .expect("unlock Vault"),
            VaultStatus::Unlocked
        );
    }

    #[test]
    fn saves_recovery_outside_the_vault_and_persists_only_its_fingerprint() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let destination = parent.path().join("CanCan Recovery.cancan-recovery");
        let runtime = VaultRuntime::new(root.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        assert!(
            !runtime
                .access_status()
                .expect("access status")
                .recovery_configured
        );

        runtime
            .save_recovery_file(&destination)
            .expect("save recovery file");

        let recovery_file = fs::read(&destination).expect("read recovery file");
        assert_eq!(&recovery_file[..8], b"CCREC001");
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&destination)
                .expect("recovery metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let recovered_key = open_recovery_file(&recovery_file).expect("recover master key");
        let store = runtime.store().expect("active store");
        assert_eq!(
            recovered_key.as_slice(),
            store.as_ref().expect("unlocked store").master_key()
        );
        drop(store);

        let status = fs::read(root.join(RECOVERY_STATUS_FILE_NAME)).expect("read status");
        assert_eq!(status.len(), RECOVERY_STATUS_MAGIC.len() + KEY_LEN);
        assert_eq!(
            &status[..RECOVERY_STATUS_MAGIC.len()],
            RECOVERY_STATUS_MAGIC
        );
        assert_eq!(
            &status[RECOVERY_STATUS_MAGIC.len()..],
            recovery_file_fingerprint(&recovery_file)
        );
        assert!(
            runtime
                .access_status()
                .expect("access status")
                .recovery_configured
        );
        assert_eq!(
            runtime
                .save_recovery_file(&parent.path().join("second.cancan-recovery"))
                .expect_err("recovery is generated once")
                .code(),
            "recovery_already_configured"
        );

        runtime.lock().expect("lock Vault");
        drop(runtime);
        let restarted = VaultRuntime::new(root);
        assert!(
            restarted
                .access_status()
                .expect("restart status")
                .recovery_configured
        );
    }

    #[test]
    fn rejects_recovery_inside_the_vault_without_marking_it_configured() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let runtime = VaultRuntime::new(root.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");

        assert_eq!(
            runtime
                .save_recovery_file(&root.join("recovery.cancan-recovery"))
                .expect_err("reject recovery inside Vault")
                .code(),
            "recovery_location_invalid"
        );
        assert!(!root.join("recovery.cancan-recovery").exists());
        assert!(
            !runtime
                .access_status()
                .expect("access status")
                .recovery_configured
        );
    }

    #[test]
    fn failed_recovery_write_does_not_mark_the_vault_configured() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");

        assert_eq!(
            runtime
                .save_recovery_file(
                    &parent
                        .path()
                        .join("missing")
                        .join("recovery.cancan-recovery"),
                )
                .expect_err("reject unavailable destination")
                .code(),
            "recovery_save_failed"
        );
        assert!(
            !runtime
                .access_status()
                .expect("access status")
                .recovery_configured
        );
    }

    #[test]
    fn system_lock_waits_for_store_cleanup_and_rejects_a_stale_store_generation() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        let stale_generation = runtime.inner.system_lock_generation.load(Ordering::SeqCst);
        let held_store = runtime.raw_store().expect("hold active store");
        let locking_runtime = runtime.clone();
        let (finished, completion) = std::sync::mpsc::channel();
        thread::spawn(move || {
            finished
                .send(locking_runtime.request_system_lock())
                .expect("send lock result");
        });

        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while runtime.system_session_active() && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        assert!(!runtime.system_session_active());
        assert!(completion.try_recv().is_err());
        drop(held_store);
        completion
            .recv_timeout(Duration::from_secs(1))
            .expect("system lock completion")
            .expect("system lock");
        assert_eq!(
            runtime.status().expect("locked status"),
            VaultStatus::Locked
        );

        runtime
            .resume_system_session()
            .expect("resume system session");
        let stale_store = runtime.store_for_system_generation(stale_generation);
        assert_eq!(
            stale_store
                .err()
                .expect("reject a pre-lock store generation")
                .code(),
            "vault_locked"
        );
    }

    #[test]
    fn remembers_unlock_in_the_secret_store_and_removes_it_explicitly() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let remembered_keys = Arc::new(MemoryRememberedKeyStore::default());
        let runtime = VaultRuntime::with_remembered_keys(root.clone(), remembered_keys.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        runtime.remember_on_this_mac().expect("remember Vault");
        assert_eq!(
            runtime.access_status().expect("remembered status"),
            VaultAccessStatus {
                recovery_configured: false,
                remembered_on_this_mac: Some(true),
                status: VaultStatus::Unlocked,
            }
        );
        runtime.lock().expect("lock Vault");
        drop(runtime);

        let restarted = VaultRuntime::with_remembered_keys(root.clone(), remembered_keys.clone());
        assert_eq!(
            restarted.access_status().expect("restart status"),
            VaultAccessStatus {
                recovery_configured: false,
                remembered_on_this_mac: Some(true),
                status: VaultStatus::Locked,
            }
        );
        assert_eq!(
            restarted
                .unlock_with_keychain()
                .expect("unlock through Keychain"),
            VaultStatus::Unlocked
        );
        restarted.forget_this_mac().expect("forget this Mac");
        restarted.lock().expect("lock forgotten Vault");
        drop(restarted);

        let forgotten = VaultRuntime::with_remembered_keys(root, remembered_keys);
        assert_eq!(
            forgotten.access_status().expect("forgotten status"),
            VaultAccessStatus {
                recovery_configured: false,
                remembered_on_this_mac: Some(false),
                status: VaultStatus::Locked,
            }
        );
        assert_eq!(
            forgotten
                .unlock_with_keychain()
                .expect_err("remembered key was removed")
                .code(),
            "remembered_unlock_unavailable"
        );
    }

    #[test]
    fn access_status_checks_presence_without_loading_the_secret() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let setup = VaultRuntime::with_remembered_keys(
            root.clone(),
            Arc::new(MemoryRememberedKeyStore::default()),
        );
        setup
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        setup.lock().expect("lock Vault");
        drop(setup);

        let runtime =
            VaultRuntime::with_remembered_keys(root, Arc::new(PresenceOnlyRememberedKeyStore));
        assert_eq!(
            runtime.access_status().expect("presence-only status"),
            VaultAccessStatus {
                recovery_configured: false,
                remembered_on_this_mac: Some(true),
                status: VaultStatus::Locked,
            }
        );
    }

    #[test]
    fn removes_a_malformed_remembered_secret_before_password_fallback() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let remembered_keys = Arc::new(MemoryRememberedKeyStore::default());
        let runtime = VaultRuntime::with_remembered_keys(root, remembered_keys.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        runtime.lock().expect("lock Vault");
        remembered_keys
            .save(&[0x55; KEY_LEN - 1])
            .expect("seed malformed secret");

        assert_eq!(
            runtime
                .unlock_with_keychain()
                .expect_err("reject malformed secret")
                .code(),
            "remembered_unlock_unavailable"
        );
        assert_eq!(
            runtime
                .access_status()
                .expect("status after malformed secret")
                .remembered_on_this_mac,
            Some(false)
        );
    }

    #[test]
    fn reports_cleanup_failure_for_a_malformed_remembered_secret() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let setup = VaultRuntime::with_remembered_keys(
            root.clone(),
            Arc::new(MemoryRememberedKeyStore::default()),
        );
        setup
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        setup.lock().expect("lock Vault");
        drop(setup);

        let runtime = VaultRuntime::with_remembered_keys(
            root,
            Arc::new(MalformedDeleteFailingRememberedKeyStore),
        );
        assert_eq!(
            runtime
                .unlock_with_keychain()
                .expect_err("surface malformed-secret cleanup failure")
                .code(),
            "remembered_unlock_failed"
        );
        assert_eq!(
            runtime
                .access_status()
                .expect("failed cleanup remains visible")
                .remembered_on_this_mac,
            Some(true)
        );
    }

    #[test]
    fn preserves_an_unverified_key_after_open_failure_and_keeps_password_unlock_available() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let remembered_keys = Arc::new(MemoryRememberedKeyStore::default());
        let runtime = VaultRuntime::with_remembered_keys(root.clone(), remembered_keys.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        runtime.lock().expect("lock Vault");
        remembered_keys
            .save(&[0x55; KEY_LEN])
            .expect("seed stale key");
        drop(runtime);

        let restarted = VaultRuntime::with_remembered_keys(root, remembered_keys);
        assert_eq!(
            restarted
                .unlock_with_keychain()
                .expect_err("reject unverified remembered key")
                .code(),
            "remembered_unlock_failed"
        );
        assert_eq!(
            restarted
                .access_status()
                .expect("status after open failure")
                .remembered_on_this_mac,
            Some(true)
        );
        assert_eq!(
            restarted
                .unlock(b"synthetic-vault-password")
                .expect("password fallback"),
            VaultStatus::Unlocked
        );
    }

    #[test]
    fn reports_a_safe_error_when_keychain_storage_fails() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let runtime = VaultRuntime::with_remembered_keys(
            parent.path().join("vault"),
            Arc::new(FailingRememberedKeyStore),
        );
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");

        assert_eq!(
            runtime
                .remember_on_this_mac()
                .expect_err("surface Keychain failure")
                .code(),
            "remember_failed"
        );
        assert_eq!(
            runtime.status().expect("Vault remains open"),
            VaultStatus::Unlocked
        );
        assert_eq!(
            runtime
                .forget_this_mac()
                .expect_err("surface Keychain deletion failure")
                .code(),
            "forget_failed"
        );
        assert_eq!(
            runtime
                .access_status()
                .expect("status remains available")
                .remembered_on_this_mac,
            None
        );
    }

    #[test]
    fn saves_updates_and_removes_one_statement_password_per_money_source() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
        let runtime =
            statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());

        runtime
            .save_statement_password("source-dbs", b"first-statement-password")
            .expect("save statement password");
        let first_state = statement_password_state(&runtime).expect("saved state");
        let secret_ref = first_state.secret_storage_key;
        assert_eq!(first_state.status, StatementPasswordStatus::Saved);
        assert_eq!(
            statement_passwords
                .load(&secret_ref)
                .expect("load statement password")
                .expect("saved statement password")
                .as_slice(),
            b"first-statement-password"
        );

        runtime
            .save_statement_password("source-dbs", b"updated-statement-password")
            .expect("replace statement password");
        let updated_state = statement_password_state(&runtime).expect("updated state");
        assert_eq!(updated_state.secret_storage_key, secret_ref);
        assert_eq!(
            statement_passwords
                .load(&secret_ref)
                .expect("load updated statement password")
                .expect("updated statement password")
                .as_slice(),
            b"updated-statement-password"
        );

        runtime
            .remove_statement_password("source-dbs")
            .expect("remove statement password");
        assert_eq!(statement_password_state(&runtime), None);
        assert!(
            statement_passwords
                .load(&secret_ref)
                .expect("load removed statement password")
                .is_none()
        );

        runtime
            .save_statement_password("source-dbs", b"locked-statement-password")
            .expect("save before lock");
        let locked_state = statement_password_state(&runtime).expect("state before lock");
        runtime.lock().expect("lock Vault");
        assert_eq!(
            runtime
                .save_statement_password("source-dbs", b"password")
                .expect_err("reject save while locked")
                .code(),
            "vault_locked"
        );
        assert_eq!(
            runtime
                .remove_statement_password("source-dbs")
                .expect_err("reject remove while locked")
                .code(),
            "vault_locked"
        );
        runtime
            .unlock(b"synthetic-vault-password")
            .expect("unlock Vault");
        assert_eq!(
            statement_password_state(&runtime).expect("state after locked commands"),
            locked_state
        );
    }

    #[test]
    fn keeps_statement_password_reference_state_consistent_when_keychain_fails() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
        let runtime =
            statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());

        statement_passwords.fail_save.store(true, Ordering::SeqCst);
        assert_eq!(
            runtime
                .save_statement_password("source-dbs", b"statement-password")
                .expect_err("surface Keychain save failure")
                .code(),
            "statement_password_save_failed"
        );
        assert_eq!(
            statement_password_state(&runtime)
                .expect("recoverable save state")
                .status,
            StatementPasswordStatus::PendingSave
        );

        statement_passwords.fail_save.store(false, Ordering::SeqCst);
        runtime
            .save_statement_password("source-dbs", b"statement-password")
            .expect("save statement password");
        statement_passwords
            .fail_delete
            .store(true, Ordering::SeqCst);
        assert_eq!(
            runtime
                .remove_statement_password("source-dbs")
                .expect_err("surface Keychain remove failure")
                .code(),
            "statement_password_remove_failed"
        );
        assert_eq!(
            statement_password_state(&runtime)
                .expect("recoverable delete state")
                .status,
            StatementPasswordStatus::PendingDelete
        );
        statement_passwords
            .fail_delete
            .store(false, Ordering::SeqCst);
        runtime
            .remove_statement_password("source-dbs")
            .expect("retry pending delete");
        assert_eq!(statement_password_state(&runtime), None);
    }

    #[test]
    fn reconciles_statement_password_crash_boundaries_after_unlock() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
        let runtime =
            statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());
        let storage_key = statement_password_storage_key("source-dbs");

        runtime
            .store()
            .expect("active store")
            .as_ref()
            .expect("unlocked store")
            .begin_statement_password_save("source-dbs", &storage_key)
            .expect("persist pending save");
        statement_passwords
            .save(&storage_key, b"unverified-different-password")
            .expect("simulate unverified Keychain write before crash");
        runtime.lock().expect("simulate process lock");
        runtime
            .unlock(b"synthetic-vault-password")
            .expect("unlock and discard unverified pending save");
        assert_eq!(statement_password_state(&runtime), None);
        assert!(
            statement_passwords
                .load(&storage_key)
                .expect("load discarded pending secret")
                .is_none()
        );

        runtime
            .save_statement_password("source-dbs", b"verified-statement-password")
            .expect("save verified password after recovery");
        runtime
            .store()
            .expect("active store")
            .as_ref()
            .expect("unlocked store")
            .begin_statement_password_delete("source-dbs")
            .expect("persist pending delete");
        statement_passwords
            .delete(&storage_key)
            .expect("simulate Keychain delete before crash");
        runtime.lock().expect("simulate second process lock");
        runtime
            .unlock(b"synthetic-vault-password")
            .expect("unlock and reconcile pending delete");
        assert_eq!(statement_password_state(&runtime), None);
    }

    #[test]
    fn unlock_paths_propagate_statement_password_reconciliation_failures() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
        let runtime =
            statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());

        runtime
            .save_statement_password("source-dbs", b"verified-statement-password")
            .expect("save statement password");
        runtime
            .store()
            .expect("active store")
            .as_ref()
            .expect("unlocked store")
            .begin_statement_password_delete("source-dbs")
            .expect("persist pending delete");
        statement_passwords
            .fail_delete
            .store(true, Ordering::SeqCst);
        runtime.lock().expect("lock Vault");

        assert_eq!(
            runtime
                .unlock(b"synthetic-vault-password")
                .expect_err("surface password-unlock reconciliation failure")
                .code(),
            "statement_password_remove_failed"
        );
        assert!(runtime.store().expect("runtime store").is_none());

        statement_passwords
            .fail_delete
            .store(false, Ordering::SeqCst);
        runtime
            .unlock(b"synthetic-vault-password")
            .expect("retry password unlock");
        assert_eq!(statement_password_state(&runtime), None);

        runtime
            .save_statement_password("source-dbs", b"replacement-statement-password")
            .expect("save replacement password");
        runtime
            .store()
            .expect("active store")
            .as_ref()
            .expect("unlocked store")
            .begin_statement_password_delete("source-dbs")
            .expect("persist second pending delete");
        runtime.remember_on_this_mac().expect("remember Vault");
        statement_passwords
            .fail_delete
            .store(true, Ordering::SeqCst);
        runtime.lock().expect("lock Vault again");

        assert_eq!(
            runtime
                .unlock_with_keychain()
                .expect_err("surface Keychain-unlock reconciliation failure")
                .code(),
            "statement_password_remove_failed"
        );
        assert!(runtime.store().expect("runtime store").is_none());

        statement_passwords
            .fail_delete
            .store(false, Ordering::SeqCst);
        runtime
            .unlock_with_keychain()
            .expect("retry Keychain unlock");
        assert_eq!(statement_password_state(&runtime), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn production_keychain_store_round_trips_binary_secret() {
        struct Cleanup(KeychainRememberedKeyStore);

        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete();
            }
        }

        let service = format!("{KEYCHAIN_SERVICE}.test.{}", candidate_name());
        let store = KeychainRememberedKeyStore::new(service.clone(), KEYCHAIN_ACCOUNT);
        store.delete().expect("remove pre-existing test entry");
        let _cleanup = Cleanup(store.clone());
        assert!(!store.is_present().expect("test entry starts absent"));

        let secret = [0_u8, 1, 2, 0, 4, 5, 6, 7];
        store.save(&secret).expect("save binary Keychain secret");
        assert!(store.is_present().expect("test entry is present"));

        let restarted = KeychainRememberedKeyStore::new(service, KEYCHAIN_ACCOUNT);
        assert_eq!(
            restarted
                .load()
                .expect("load Keychain secret")
                .expect("saved secret exists")
                .as_slice(),
            secret
        );
        restarted.delete().expect("delete Keychain secret");
        assert!(!restarted.is_present().expect("test entry is absent"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn production_statement_password_store_replaces_and_removes_secret() {
        struct Cleanup {
            secret_ref: String,
            store: KeychainStatementPasswordStore,
        }

        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.store.delete(&self.secret_ref);
            }
        }

        let service = format!(
            "{STATEMENT_PASSWORD_KEYCHAIN_SERVICE}.test.{}",
            candidate_name()
        );
        let secret_ref = random_identifier("statement-password-test");
        let store = KeychainStatementPasswordStore::new(service);
        store
            .delete(&secret_ref)
            .expect("remove pre-existing test entry");
        let _cleanup = Cleanup {
            secret_ref: secret_ref.clone(),
            store: store.clone(),
        };

        store
            .save(&secret_ref, b"first-synthetic-password")
            .expect("save statement password");
        store
            .save(&secret_ref, b"updated-synthetic-password")
            .expect("replace statement password");
        assert_eq!(
            store
                .load(&secret_ref)
                .expect("load statement password")
                .expect("saved statement password exists")
                .as_slice(),
            b"updated-synthetic-password"
        );
        store
            .delete(&secret_ref)
            .expect("delete statement password");
        assert!(
            store
                .load(&secret_ref)
                .expect("load deleted statement password")
                .is_none()
        );
    }

    #[test]
    fn rejects_empty_passwords_and_existing_or_corrupt_vaults() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let runtime = VaultRuntime::new(root.clone());

        assert_eq!(
            runtime
                .create(b"")
                .expect_err("reject empty password")
                .code(),
            "password_required"
        );
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        runtime.lock().expect("lock Vault");
        assert_eq!(
            runtime
                .create(b"another-password")
                .expect_err("reject existing Vault")
                .code(),
            "vault_already_exists"
        );

        fs::write(root.join(KEY_FILE_NAME), b"corrupt wrapper").expect("corrupt wrapper fixture");
        assert_eq!(
            runtime
                .unlock(b"synthetic-vault-password")
                .expect_err("reject corrupt wrapper")
                .code(),
            "invalid_vault"
        );
        assert_eq!(
            runtime.status().expect_err("corrupt status").code(),
            "invalid_vault"
        );
    }

    #[test]
    fn serializes_status_with_a_concurrent_create_transition() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        let creating = runtime.clone();
        let create = thread::spawn(move || creating.create(b"synthetic-vault-password"));

        let mut observed_transition = false;
        for _ in 0..100 {
            match runtime.inner.store.try_lock() {
                Err(std::sync::TryLockError::WouldBlock) => {
                    observed_transition = true;
                    break;
                }
                Err(std::sync::TryLockError::Poisoned(_)) => panic!("runtime mutex poisoned"),
                Ok(guard) => drop(guard),
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            observed_transition,
            "create never acquired the session mutex"
        );
        assert_eq!(
            runtime.status().expect("status after serialized create"),
            VaultStatus::Unlocked
        );
        assert_eq!(
            create.join().expect("create thread").expect("create Vault"),
            VaultStatus::Unlocked
        );
    }

    #[test]
    fn classifies_a_missing_wrapper_consistently_as_an_invalid_vault() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        fs::create_dir(&root).expect("create incomplete Vault");
        let runtime = VaultRuntime::new(root);

        assert_eq!(
            runtime.status().expect_err("invalid status").code(),
            "invalid_vault"
        );
        assert_eq!(
            runtime
                .unlock(b"synthetic-vault-password")
                .expect_err("invalid unlock")
                .code(),
            "invalid_vault"
        );
        assert_eq!(
            runtime.lock().expect_err("invalid lock").code(),
            "invalid_vault"
        );
    }

    #[test]
    fn rejects_unlock_when_the_existing_vault_database_is_missing() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let runtime = VaultRuntime::new(root.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        runtime.lock().expect("lock Vault");
        let database_path = root.join(DATABASE_FILE_NAME);
        fs::remove_file(&database_path).expect("remove Vault database");

        assert_eq!(
            runtime
                .status()
                .expect_err("reject incomplete status")
                .code(),
            "invalid_vault"
        );
        assert_eq!(
            runtime
                .unlock(b"synthetic-vault-password")
                .expect_err("reject incomplete Vault")
                .code(),
            "invalid_vault"
        );
        assert!(
            !database_path.exists(),
            "unlock must not recreate the database"
        );
    }

    #[test]
    fn reopens_an_activated_vault_as_locked_after_runtime_restart() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let root = parent.path().join("vault");
        let runtime = VaultRuntime::new(root.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        drop(runtime);

        let restarted = VaultRuntime::new(root);
        assert_eq!(
            restarted.status().expect("restart status"),
            VaultStatus::Locked
        );
        assert_eq!(
            restarted
                .unlock(b"synthetic-vault-password")
                .expect("unlock restarted Vault"),
            VaultStatus::Unlocked
        );
    }

    #[test]
    fn imports_and_lists_an_unassigned_document_only_while_unlocked() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let source_path = parent.path().join("DBS-July-2026.pdf");
        fs::write(&source_path, synthetic_pdf()).expect("write statement fixture");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        let imported = runtime
            .import_selected_document(&source_path, None)
            .expect("import statement");
        assert_eq!(imported.status, SourceDocumentImportStatus::Imported);

        let duplicate = runtime
            .import_selected_document(&source_path, None)
            .expect("deduplicate statement");
        assert_eq!(duplicate.document_id, imported.document_id);
        assert_eq!(duplicate.status, SourceDocumentImportStatus::AlreadyPresent);

        runtime
            .delete_source_document(&imported.document_id)
            .expect("delete encrypted source");
        assert!(
            source_path.exists(),
            "user-selected source must remain untouched"
        );
        let deleted = runtime
            .list_unassigned_source_documents()
            .expect("list deleted document");
        assert_eq!(deleted[0].file_state, "deleted");
        let confirmation = runtime
            .import_selected_document(&source_path, None)
            .expect("request restore confirmation");
        assert_eq!(
            confirmation.status,
            SourceDocumentImportStatus::RestoreConfirmationRequired
        );
        let restored = runtime
            .import_selected_document(&source_path, Some(&imported.document_id))
            .expect("restore deleted source");
        assert_eq!(restored.document_id, imported.document_id);
        assert_eq!(restored.status, SourceDocumentImportStatus::Restored);

        let documents = runtime
            .list_unassigned_source_documents()
            .expect("list unassigned documents");
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].document_id, imported.document_id);
        assert_eq!(documents[0].original_filename, "DBS-July-2026.pdf");
        assert_eq!(documents[0].mime_type, "application/pdf");
        assert_eq!(documents[0].byte_size, synthetic_pdf().len() as u64);
        assert_eq!(documents[0].file_state, "available");
        {
            let store = runtime.inner.store.lock().expect("runtime store");
            let persisted = store
                .as_ref()
                .expect("unlocked store")
                .list_unassigned_documents()
                .expect("inspect imported document");
            assert_eq!(persisted[0].money_source_id, None);
            assert_eq!(persisted[0].semantic_document_key, None);
        }

        runtime.lock().expect("lock Vault");
        assert_eq!(
            runtime
                .list_unassigned_source_documents()
                .expect_err("reject list while locked")
                .code(),
            "vault_locked"
        );
        assert_eq!(
            runtime
                .import_selected_document(&source_path, None)
                .expect_err("reject import while locked")
                .code(),
            "vault_locked"
        );
    }

    #[test]
    fn rejects_unsupported_document_imports() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        let unsupported = parent.path().join("statement.exe");
        fs::write(&unsupported, b"not a financial document").expect("write unsupported fixture");

        assert_eq!(
            runtime
                .import_selected_document(&unsupported, None)
                .expect_err("reject unsupported document")
                .code(),
            "unsupported_document"
        );
    }

    #[test]
    fn renders_an_imported_pdf_in_memory_only_while_the_vault_is_unlocked() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let source_path = parent.path().join("statement.pdf");
        fs::write(&source_path, synthetic_pdf()).expect("write PDF fixture");
        let vault_root = parent.path().join("vault");
        let runtime = VaultRuntime::new(vault_root.clone());
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        let imported = runtime
            .import_selected_document(&source_path, None)
            .expect("import PDF");
        let before = vault_entries(&vault_root);

        let rendered = runtime
            .render_source_document_page(&imported.document_id, 1)
            .expect("render PDF");

        assert_eq!(rendered.page_count, 1);
        assert_eq!(rendered.page_number, 1);
        assert!(!rendered.png_base64.is_empty());
        assert_eq!(vault_entries(&vault_root), before);
        runtime.lock().expect("lock Vault");
        assert_eq!(
            runtime
                .render_source_document_page(&imported.document_id, 1)
                .expect_err("reject rendering while locked")
                .code(),
            "vault_locked"
        );
    }

    #[test]
    fn rejects_non_pdf_and_invalid_page_view_requests() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let csv_path = parent.path().join("statement.csv");
        fs::write(&csv_path, b"date,amount\n2026-07-19,42").expect("write CSV fixture");
        let pdf_path = parent.path().join("statement.pdf");
        fs::write(&pdf_path, synthetic_pdf()).expect("write PDF fixture");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        let csv = runtime
            .import_selected_document(&csv_path, None)
            .expect("import CSV");
        let pdf = runtime
            .import_selected_document(&pdf_path, None)
            .expect("import PDF");

        assert_eq!(
            runtime
                .render_source_document_page(&csv.document_id, 1)
                .expect_err("reject CSV viewer")
                .code(),
            "viewer_unsupported"
        );
        assert_eq!(
            runtime
                .render_source_document_page(&pdf.document_id, 0)
                .expect_err("reject page zero")
                .code(),
            "invalid_document_request"
        );
        assert_eq!(
            runtime
                .render_source_document_page(&pdf.document_id, 2)
                .expect_err("reject out-of-range page")
                .code(),
            "invalid_document_request"
        );
        assert_eq!(
            runtime
                .render_source_document_page("missing-document", 1)
                .expect_err("reject missing document")
                .code(),
            "document_unavailable"
        );
    }

    #[test]
    fn previews_only_bounded_csv_lines_while_the_vault_is_unlocked() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");

        let csv_path = parent.path().join("wise-export.csv");
        fs::write(
            &csv_path,
            b"date,amount\r\n2026-07-01,10.00\r\n2026-07-02,-4.25\n",
        )
        .expect("write CSV fixture");
        let imported = runtime
            .import_selected_document(&csv_path, None)
            .expect("import CSV");
        let preview = runtime
            .preview_source_document(&imported.document_id)
            .expect("preview CSV");
        assert_eq!(preview.line_count, 3);
        assert_eq!(preview.preview_lines, 3);
        assert_eq!(
            preview.preview_text,
            "date,amount\n2026-07-01,10.00\n2026-07-02,-4.25"
        );
        assert!(!preview.truncated);

        let mut big_content = String::from("date,amount\n");
        for row in 1..=300 {
            big_content.push_str(&format!("2026-07-01,{row}.00\n"));
        }
        let big_path = parent.path().join("big-export.csv");
        fs::write(&big_path, big_content).expect("write big CSV fixture");
        let big = runtime
            .import_selected_document(&big_path, None)
            .expect("import big CSV");
        let big_preview = runtime
            .preview_source_document(&big.document_id)
            .expect("preview big CSV");
        assert_eq!(big_preview.line_count, 301);
        assert_eq!(big_preview.preview_lines, PREVIEW_MAX_LINES as u64);
        assert!(big_preview.truncated);
        assert_eq!(big_preview.preview_text.lines().count(), PREVIEW_MAX_LINES);
        assert!(big_preview.preview_text.starts_with("date,amount\n"));

        let long_path = parent.path().join("long-line.csv");
        let long_content = format!("memo,{}\n2026-07-02,1.00\n", "é".repeat(PREVIEW_MAX_BYTES));
        fs::write(&long_path, long_content).expect("write long-line CSV fixture");
        let long = runtime
            .import_selected_document(&long_path, None)
            .expect("import long-line CSV");
        let long_preview = runtime
            .preview_source_document(&long.document_id)
            .expect("preview long-line CSV");
        assert_eq!(long_preview.line_count, 2);
        assert_eq!(long_preview.preview_lines, 1);
        assert!(long_preview.truncated);
        assert!(long_preview.preview_text.len() <= PREVIEW_MAX_BYTES);
        assert!(long_preview.preview_text.starts_with("memo,"));

        // The byte budget is never exceeded, even by a separator byte or a
        // zero-byte trailing line.
        let capped_path = parent.path().join("capped.csv");
        let capped_content = format!("{}\n\n", "x".repeat(PREVIEW_MAX_BYTES));
        fs::write(&capped_path, capped_content).expect("write capped CSV fixture");
        let capped = runtime
            .import_selected_document(&capped_path, None)
            .expect("import capped CSV");
        let capped_preview = runtime
            .preview_source_document(&capped.document_id)
            .expect("preview capped CSV");
        assert_eq!(capped_preview.line_count, 2);
        assert_eq!(capped_preview.preview_lines, 1);
        assert!(capped_preview.truncated);
        assert_eq!(capped_preview.preview_text.len(), PREVIEW_MAX_BYTES);

        let empty_path = parent.path().join("empty.csv");
        fs::write(&empty_path, b"").expect("write empty CSV fixture");
        let empty = runtime
            .import_selected_document(&empty_path, None)
            .expect("import empty CSV");
        let empty_preview = runtime
            .preview_source_document(&empty.document_id)
            .expect("preview empty CSV");
        assert_eq!(empty_preview.line_count, 0);
        assert_eq!(empty_preview.preview_lines, 0);
        assert!(!empty_preview.truncated);
        assert!(empty_preview.preview_text.is_empty());

        let pdf_path = parent.path().join("statement.pdf");
        fs::write(&pdf_path, synthetic_pdf()).expect("write PDF fixture");
        let pdf = runtime
            .import_selected_document(&pdf_path, None)
            .expect("import PDF");
        assert_eq!(
            runtime
                .preview_source_document(&pdf.document_id)
                .expect_err("PDF keeps the pixel viewer")
                .code(),
            "viewer_unsupported"
        );
        assert_eq!(
            runtime
                .preview_source_document("missing-document")
                .expect_err("reject missing document")
                .code(),
            "document_unavailable"
        );

        runtime.lock().expect("lock Vault");
        assert_eq!(
            runtime
                .preview_source_document(&imported.document_id)
                .expect_err("reject preview while locked")
                .code(),
            "vault_locked"
        );
    }

    #[test]
    fn applies_verified_mock_normalizer_routing_without_renderer_identity_input() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let source_path = parent.path().join("synthetic.csv");
        let fixture = [
            "CANCAN_SYNTHETIC_STATEMENT_V1",
            "provider=synthetic-bank",
            "statement_id=transfer-2026-07",
        ]
        .join("\n");
        fs::write(&source_path, &fixture).expect("write statement fixture");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        runtime
            .seed_money_source(
                "source-synthetic",
                "synthetic-bank",
                "Synthetic Bank",
                "bank",
            )
            .expect("seed source");
        let imported = runtime
            .import_selected_document(&source_path, None)
            .expect("capture statement");
        let result = NormalizerResult::Classified {
            proposal: NormalizerProposal {
                document: NormalizerDocument {
                    document_type: "transfer_export".to_owned(),
                    provider_key: "synthetic-bank".to_owned(),
                    statement_id: Some("transfer-2026-07".to_owned()),
                },
                accounts: vec![NormalizerAccount {
                    account_type: "deposit_account".to_owned(),
                    currency: Some("SGD".to_owned()),
                    masked_identifier: Some("••001".to_owned()),
                    provider_account_id: Some("checking-001".to_owned()),
                }],
            },
        };
        let routed = runtime
            .apply_normalizer_result(&imported.document_id, &fixture, result)
            .expect("apply trusted routing");

        assert_eq!(
            routed.status,
            crate::database::SourceDocumentRoutingStatus::Routed
        );
        assert_eq!(routed.money_source_id.as_deref(), Some("source-synthetic"));
        assert_eq!(routed.account_ids.len(), 1);
        assert!(
            runtime
                .list_unassigned_source_documents()
                .expect("list pending")
                .is_empty()
        );
    }

    fn synthetic_pdf() -> Vec<u8> {
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 64 96] /Resources << >> /Contents 4 0 R >>",
            "<< /Length 23 >>\nstream\n0 0 0 rg 0 0 64 96 re f\nendstream",
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::with_capacity(objects.len());
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            write!(&mut pdf, "{} 0 obj\n{}\nendobj\n", index + 1, object)
                .expect("write PDF object");
        }
        let xref = pdf.len();
        write!(
            &mut pdf,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len() + 1
        )
        .expect("write xref");
        for offset in offsets {
            writeln!(&mut pdf, "{offset:010} 00000 n ").expect("write xref entry");
        }
        write!(
            &mut pdf,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .expect("write trailer");
        pdf
    }

    fn vault_entries(root: &Path) -> Vec<PathBuf> {
        fn visit(root: &Path, path: &Path, entries: &mut Vec<PathBuf>) {
            for entry in fs::read_dir(path).expect("read Vault directory") {
                let entry = entry.expect("read Vault entry");
                let entry_path = entry.path();
                entries.push(
                    entry_path
                        .strip_prefix(root)
                        .expect("Vault-relative path")
                        .to_owned(),
                );
                if entry.file_type().expect("Vault entry type").is_dir() {
                    visit(root, &entry_path, entries);
                }
            }
        }

        let mut entries = Vec::new();
        visit(root, root, &mut entries);
        entries.sort();
        entries
    }
}
