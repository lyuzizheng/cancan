use crate::{
    database::{
        DATABASE_FILE_NAME, ManualImportStore, SourceDocumentFileInput, SourceDocumentImport,
        SourceDocumentImportOutcome, SourceDocumentRoutingOutcome, TrustedAccountCandidate,
        TrustedDocumentClassification,
    },
    vault::{create_password_wrapper, open_password_wrapper, password_wrapper_profile},
    viewer::{RenderedDocumentPage, render_pdf_page},
};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::{
    ShellExt,
    process::{CommandChild, CommandEvent},
};
use tokio::time::{Instant, timeout_at};
use zeroize::Zeroizing;

const KEY_LEN: usize = 32;
const KEY_FILE_NAME: &str = "vault-key.ccenv";
const IMPORT_POLICY_VERSION: &str = "manual-import-v1";
const NORMALIZER_TIMEOUT: Duration = Duration::from_secs(10);
const NORMALIZER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VaultStatus {
    NotCreated,
    Locked,
    Unlocked,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct VaultCommandError {
    code: &'static str,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceDocumentSummary {
    byte_size: u64,
    document_id: String,
    file_state: String,
    mime_type: String,
    original_filename: String,
    received_at: String,
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
    root: PathBuf,
    store: Mutex<Option<ManualImportStore>>,
}

impl VaultRuntime {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                root,
                store: Mutex::new(None),
            }),
        }
    }

    pub(crate) fn status(&self) -> Result<VaultStatus, RuntimeError> {
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
        *store = Some(opened);
        Ok(VaultStatus::Unlocked)
    }

    pub(crate) fn lock(&self) -> Result<VaultStatus, RuntimeError> {
        let mut store = self.store()?;
        *store = None;
        self.locked_status()
    }

    pub(crate) fn import_selected_document(
        &self,
        source_path: &Path,
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
            .register_import(&input)
            .map_err(|_| RuntimeError::new("import_failed"))
    }

    fn require_unlocked(&self) -> Result<(), RuntimeError> {
        if self.store()?.is_none() {
            return Err(RuntimeError::new("vault_locked"));
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
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .list_documents(money_source_id)
            .map(|documents| {
                documents
                    .into_iter()
                    .map(|document| SourceDocumentSummary {
                        byte_size: document.byte_size,
                        document_id: document.document_id,
                        file_state: document.file_state,
                        mime_type: document.mime_type,
                        original_filename: document.original_filename,
                        received_at: document.received_at,
                    })
                    .collect()
            })
            .map_err(|_| RuntimeError::new("list_documents_failed"))
    }

    pub(crate) fn list_unassigned_source_documents(
        &self,
    ) -> Result<Vec<SourceDocumentSummary>, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .list_unassigned_documents()
            .map(|documents| {
                documents
                    .into_iter()
                    .map(|document| SourceDocumentSummary {
                        byte_size: document.byte_size,
                        document_id: document.document_id,
                        file_state: document.file_state,
                        mime_type: document.mime_type,
                        original_filename: document.original_filename,
                        received_at: document.received_at,
                    })
                    .collect()
            })
            .map_err(|_| RuntimeError::new("list_documents_failed"))
    }

    fn normalization_input(
        &self,
        document_id: &str,
    ) -> Result<SourceDocumentFileInput, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .source_document_input(document_id)
            .map_err(|_| RuntimeError::new("document_unavailable"))
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
        render_pdf_page(&input.plaintext, page_number).map_err(document_render_error)
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
        self.inner
            .store
            .lock()
            .map_err(|_| RuntimeError::new("runtime_unavailable"))
    }
}

fn document_render_error(error: io::Error) -> RuntimeError {
    match error.kind() {
        io::ErrorKind::InvalidInput => RuntimeError::new("invalid_document_request"),
        io::ErrorKind::Unsupported => RuntimeError::new("viewer_unsupported"),
        _ => RuntimeError::new("document_render_failed"),
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
pub(crate) async fn import_source_document(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<Option<SourceDocumentImportOutcome>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
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
        runtime.import_selected_document(&path).map(Some)
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
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        runtime.render_source_document_page(&document_id, page_number)
    })
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
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::SourceDocumentImportStatus;
    use std::{thread, time::Duration};

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
        fs::write(&source_path, b"%PDF synthetic statement").expect("write statement fixture");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        let imported = runtime
            .import_selected_document(&source_path)
            .expect("import statement");
        assert_eq!(imported.status, SourceDocumentImportStatus::Imported);

        let duplicate = runtime
            .import_selected_document(&source_path)
            .expect("deduplicate statement");
        assert_eq!(duplicate.document_id, imported.document_id);
        assert_eq!(duplicate.status, SourceDocumentImportStatus::AlreadyPresent);

        let documents = runtime
            .list_unassigned_source_documents()
            .expect("list unassigned documents");
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].document_id, imported.document_id);
        assert_eq!(documents[0].original_filename, "DBS-July-2026.pdf");
        assert_eq!(documents[0].mime_type, "application/pdf");
        assert_eq!(documents[0].byte_size, 24);
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
                .import_selected_document(&source_path)
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
                .import_selected_document(&unsupported)
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
            .import_selected_document(&source_path)
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
            .import_selected_document(&csv_path)
            .expect("import CSV");
        let pdf = runtime
            .import_selected_document(&pdf_path)
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
            .import_selected_document(&source_path)
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
