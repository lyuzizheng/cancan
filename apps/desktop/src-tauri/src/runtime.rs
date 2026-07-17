use crate::{
    database::{
        DATABASE_FILE_NAME, ManualImportStore, SourceDocumentImport, SourceDocumentImportOutcome,
    },
    vault::{create_password_wrapper, open_password_wrapper, password_wrapper_profile},
};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use zeroize::Zeroizing;

const KEY_LEN: usize = 32;
const KEY_FILE_NAME: &str = "vault-key.ccenv";
const IMPORT_POLICY_VERSION: &str = "manual-import-v1";

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportSourceDocumentRequest {
    money_source_id: String,
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
        request: &ImportSourceDocumentRequest,
        source_path: &Path,
    ) -> Result<SourceDocumentImportOutcome, RuntimeError> {
        validate_import_request(request)?;
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
            money_source_id: &request.money_source_id,
            original_filename: &original_filename,
            semantic_document_key: None,
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
    request: ImportSourceDocumentRequest,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<Option<SourceDocumentImportOutcome>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        validate_import_request(&request)?;
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
        runtime.import_selected_document(&request, &path).map(Some)
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

fn validate_import_request(request: &ImportSourceDocumentRequest) -> Result<(), RuntimeError> {
    if request.money_source_id.is_empty() {
        return Err(RuntimeError::new("invalid_import_request"));
    }
    Ok(())
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
    fn imports_and_lists_a_selected_document_only_while_unlocked() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let source_path = parent.path().join("DBS-July-2026.pdf");
        fs::write(&source_path, b"%PDF synthetic statement").expect("write statement fixture");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        runtime
            .seed_money_source("source-dbs", "dbs", "DBS", "bank")
            .expect("seed source");
        let request = ImportSourceDocumentRequest {
            money_source_id: "source-dbs".to_owned(),
        };

        let imported = runtime
            .import_selected_document(&request, &source_path)
            .expect("import statement");
        assert_eq!(imported.status, SourceDocumentImportStatus::Imported);

        let duplicate = runtime
            .import_selected_document(&request, &source_path)
            .expect("deduplicate statement");
        assert_eq!(duplicate.document_id, imported.document_id);
        assert_eq!(duplicate.status, SourceDocumentImportStatus::AlreadyPresent);

        let documents = runtime
            .list_source_documents("source-dbs")
            .expect("list documents");
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
                .list_documents("source-dbs")
                .expect("inspect imported document");
            assert_eq!(persisted[0].semantic_document_key, None);
        }

        runtime.lock().expect("lock Vault");
        assert_eq!(
            runtime
                .list_source_documents("source-dbs")
                .expect_err("reject list while locked")
                .code(),
            "vault_locked"
        );
        assert_eq!(
            runtime
                .import_selected_document(&request, &source_path)
                .expect_err("reject import while locked")
                .code(),
            "vault_locked"
        );
    }

    #[test]
    fn rejects_unsupported_or_incomplete_document_imports() {
        let parent = tempfile::tempdir().expect("temporary app data");
        let runtime = VaultRuntime::new(parent.path().join("vault"));
        runtime
            .create(b"synthetic-vault-password")
            .expect("create Vault");
        runtime
            .seed_money_source("source-dbs", "dbs", "DBS", "bank")
            .expect("seed source");
        let unsupported = parent.path().join("statement.exe");
        fs::write(&unsupported, b"not a financial document").expect("write unsupported fixture");

        assert_eq!(
            runtime
                .import_selected_document(
                    &ImportSourceDocumentRequest {
                        money_source_id: "source-dbs".to_owned(),
                    },
                    &unsupported,
                )
                .expect_err("reject unsupported document")
                .code(),
            "unsupported_document"
        );
        assert_eq!(
            runtime
                .import_selected_document(
                    &ImportSourceDocumentRequest {
                        money_source_id: String::new(),
                    },
                    &parent.path().join("missing.pdf"),
                )
                .expect_err("reject incomplete request")
                .code(),
            "invalid_import_request"
        );
    }
}
