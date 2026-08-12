use super::*;
use tauri::Emitter;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecoveryStatus {
    Configured { remind_after: Option<u64> },
    Postponed { remind_after: u64 },
}

impl VaultRuntime {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self::with_secret_stores(
            root,
            Arc::new(KeychainRememberedKeyStore::production()),
            Arc::new(KeychainStatementPasswordStore::production()),
            Arc::new(KeychainLocalInboxBookmarkStore::production()),
        )
    }

    #[cfg(test)]
    pub(super) fn with_remembered_keys(
        root: PathBuf,
        remembered_keys: Arc<dyn RememberedKeyStore>,
    ) -> Self {
        Self::with_secret_stores(
            root,
            remembered_keys,
            Arc::new(KeychainStatementPasswordStore::production()),
            Arc::new(KeychainLocalInboxBookmarkStore::production()),
        )
    }

    pub(super) fn with_secret_stores(
        root: PathBuf,
        remembered_keys: Arc<dyn RememberedKeyStore>,
        statement_passwords: Arc<dyn StatementPasswordStore>,
        local_inbox_bookmarks: Arc<dyn LocalInboxBookmarkStore>,
    ) -> Self {
        Self::with_all_secret_stores(
            root,
            remembered_keys,
            statement_passwords,
            local_inbox_bookmarks,
            Arc::new(KeychainGmailRefreshTokenStore::production()),
        )
    }

    pub(super) fn with_all_secret_stores(
        root: PathBuf,
        remembered_keys: Arc<dyn RememberedKeyStore>,
        statement_passwords: Arc<dyn StatementPasswordStore>,
        local_inbox_bookmarks: Arc<dyn LocalInboxBookmarkStore>,
        gmail_refresh_tokens: Arc<dyn GmailRefreshTokenStore>,
    ) -> Self {
        let runtime = Self {
            inner: Arc::new(RuntimeInner {
                document_passwords: Mutex::new(HashMap::new()),
                gmail_refresh_tokens,
                local_inbox_access: Mutex::new(None),
                local_inbox_bookmarks,
                local_inbox_last_scan: Mutex::new(None),
                local_inbox_scan_guard: Mutex::new(()),
                local_inbox_watcher: Mutex::new(None),
                local_inbox_needs_attention: AtomicBool::new(false),
                local_inbox_needs_reauthorization: AtomicBool::new(false),
                remembered_keys,
                root,
                statement_passwords,
                store: Mutex::new(None),
                vault_session_generation: AtomicU64::new(0),
                #[cfg(test)]
                intake_test_hooks: Mutex::new(IntakeTestHooks::default()),
            }),
        };
        if let Some(parent) = runtime.inner.root.parent() {
            cleanup_inactive_vault_candidates(parent);
        }
        runtime
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

    pub(super) fn recovery_configured(&self) -> bool {
        matches!(
            self.read_recovery_status_file(),
            Some(RecoveryStatus::Configured { .. })
        )
    }

    pub(super) fn recovery_reminder(&self) -> Option<u64> {
        match self.read_recovery_status_file()? {
            RecoveryStatus::Configured { remind_after } => remind_after,
            RecoveryStatus::Postponed { remind_after } => Some(remind_after),
        }
    }

    #[cfg(test)]
    pub(crate) fn postpone_recovery_setup(&self) -> Result<(), RuntimeError> {
        let status = self.read_recovery_status_file();
        let mut fingerprint = [0_u8; KEY_LEN];
        let include_fingerprint = matches!(status, Some(RecoveryStatus::Configured { .. }));
        if include_fingerprint {
            let existing = fs::read(self.inner.root.join(RECOVERY_STATUS_FILE_NAME))
                .map_err(|_| RuntimeError::new("recovery_status_read_failed"))?;
            fingerprint.copy_from_slice(
                &existing[RECOVERY_STATUS_MAGIC.len()..RECOVERY_STATUS_MAGIC.len() + KEY_LEN],
            );
        }
        let remind_after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| RuntimeError::new("clock_error"))?
            .as_secs()
            .saturating_add(7 * 24 * 60 * 60);
        let mut bytes = Vec::with_capacity(8 + KEY_LEN + 8);
        bytes.extend_from_slice(RECOVERY_STATUS_V2_MAGIC);
        if include_fingerprint {
            bytes.extend_from_slice(&fingerprint);
        }
        bytes.extend_from_slice(&remind_after.to_le_bytes());
        write_atomic(&self.inner.root.join(RECOVERY_STATUS_FILE_NAME), &bytes)
            .map_err(|_| RuntimeError::new("recovery_status_write_failed"))
    }

    fn read_recovery_status_file(&self) -> Option<RecoveryStatus> {
        let status = fs::read(self.inner.root.join(RECOVERY_STATUS_FILE_NAME)).ok()?;
        if status.len() == RECOVERY_STATUS_MAGIC.len() + KEY_LEN
            && &status[..RECOVERY_STATUS_MAGIC.len()] == RECOVERY_STATUS_MAGIC
        {
            return Some(RecoveryStatus::Configured { remind_after: None });
        }
        if status.len() == RECOVERY_STATUS_V2_MAGIC.len() + KEY_LEN + 8
            && &status[..RECOVERY_STATUS_V2_MAGIC.len()] == RECOVERY_STATUS_V2_MAGIC
        {
            let remind_after =
                u64_from_le_bytes(&status[RECOVERY_STATUS_V2_MAGIC.len() + KEY_LEN..]);
            return Some(RecoveryStatus::Configured {
                remind_after: Some(remind_after),
            });
        }
        if status.len() == RECOVERY_STATUS_V2_MAGIC.len() + 8
            && &status[..RECOVERY_STATUS_V2_MAGIC.len()] == RECOVERY_STATUS_V2_MAGIC
        {
            let remind_after = u64_from_le_bytes(&status[RECOVERY_STATUS_V2_MAGIC.len()..]);
            return Some(RecoveryStatus::Postponed { remind_after });
        }
        None
    }

    pub(crate) fn status(&self) -> Result<VaultStatus, RuntimeError> {
        let store = self.store()?;
        if store.is_some() {
            return Ok(VaultStatus::Unlocked);
        }
        self.locked_status()
    }

    pub(super) fn locked_status(&self) -> Result<VaultStatus, RuntimeError> {
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
        drop(store);
        self.advance_vault_session();
        let _ = self.activate_local_inbox_from_bookmark();
        Ok(VaultStatus::Unlocked)
    }

    pub(super) fn create_candidate(
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

    pub(super) fn rollback_activation(&self, candidate: &Path, parent: &Path) -> Result<(), ()> {
        fs::rename(&self.inner.root, candidate).map_err(|_| ())?;
        sync_directory(parent).map_err(|_| ())
    }

    fn finish_unlock(
        &self,
        opened: ManualImportStore,
        mut store: MutexGuard<'_, Option<ManualImportStore>>,
    ) -> Result<VaultStatus, RuntimeError> {
        self.reconcile_statement_passwords(&opened)?;
        self.reconcile_gmail_accounts_after_unlock(&opened);
        *store = Some(opened);
        drop(store);
        self.advance_vault_session();
        let _ = self.activate_local_inbox_from_bookmark();
        Ok(VaultStatus::Unlocked)
    }

    pub(crate) fn unlock(&self, password: &[u8]) -> Result<VaultStatus, RuntimeError> {
        if password.is_empty() {
            return Err(RuntimeError::new("password_required"));
        }
        let store = self.store()?;
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
        self.finish_unlock(opened, store)
    }

    pub(crate) fn unlock_with_keychain(&self) -> Result<VaultStatus, RuntimeError> {
        // Load the key before acquiring the store guard: the Touch ID-protected
        // read shows a system prompt that can stay up until the user responds,
        // and holding the store mutex across it would block status queries and
        // background intake jobs for the entire prompt.
        let Some(master_key) = self
            .load_remembered_master_key()
            .map_err(|_| RuntimeError::new("remembered_unlock_failed"))?
        else {
            return Err(RuntimeError::new("remembered_unlock_unavailable"));
        };
        let store = self.store()?;
        if store.is_some() {
            return Ok(VaultStatus::Unlocked);
        }
        if self.locked_status()? != VaultStatus::Locked {
            return Err(RuntimeError::new("vault_not_created"));
        }
        let opened = ManualImportStore::open_existing(&self.inner.root, master_key)
            .map_err(|_| RuntimeError::new("remembered_unlock_failed"))?;
        self.finish_unlock(opened, store)
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
        self.clear_local_inbox_access();
        let mut store = self.store()?;
        *store = None;
        self.document_passwords()?.clear();
        self.locked_status()
    }
}

pub(super) fn cleanup_inactive_vault_candidates(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    let prefix = VAULT_CANDIDATE_PREFIX;
    let prefix_len = prefix.len();
    let expected_len = prefix_len + 16;
    let mut removed_any = false;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = match file_name.to_str() {
            Some(name) => name,
            None => continue,
        };
        if name.len() != expected_len
            || !name.starts_with(prefix)
            || !name[prefix_len..]
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            continue;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if fs::remove_dir_all(&path).is_ok() {
            removed_any = true;
        }
    }
    if removed_any {
        let _ = sync_directory(parent);
    }
}

#[tauri::command]
pub(crate) async fn vault_status(
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.status()).await
}

#[tauri::command]
pub(crate) async fn vault_access_status(
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultAccessStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.access_status()).await
}

#[tauri::command]
pub(crate) async fn create_vault(
    password: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let password = Zeroizing::new(password);
    run_runtime_task(move || runtime.create(password.as_bytes())).await
}

async fn unlock_and_resume<F>(
    app: AppHandle,
    runtime: VaultRuntime,
    unlock: F,
) -> Result<VaultStatus, VaultCommandError>
where
    F: FnOnce() -> Result<VaultStatus, RuntimeError> + Send + 'static,
{
    let status = tauri::async_runtime::spawn_blocking(unlock)
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        .map_err(VaultCommandError::from)?;
    resume_review_jobs_after_unlock(&app, runtime.clone()).await;
    resume_parse_document_jobs_after_unlock(&app, runtime.clone()).await;
    resume_document_reconciliations_after_unlock(runtime.clone()).await;
    resume_local_inbox_after_unlock(&app, runtime).await;
    Ok(status)
}

#[tauri::command]
pub(crate) async fn unlock_vault(
    password: String,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let password = Zeroizing::new(password);
    let unlock_runtime = runtime.clone();
    unlock_and_resume(app, runtime, move || {
        unlock_runtime.unlock(password.as_bytes())
    })
    .await
}

#[tauri::command]
pub(crate) async fn unlock_vault_with_keychain(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let unlock_runtime = runtime.clone();
    unlock_and_resume(app, runtime, move || unlock_runtime.unlock_with_keychain()).await
}

#[tauri::command]
pub(crate) async fn remember_vault_on_this_mac(
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.remember_on_this_mac()).await
}

#[tauri::command]
pub(crate) async fn forget_vault_on_this_mac(
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.forget_this_mac()).await
}

#[tauri::command]
pub(crate) async fn lock_vault(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<VaultStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let status = run_runtime_task(move || runtime.lock()).await?;
    let _ = app.emit("vault-locked", ());
    Ok(status)
}

#[tauri::command]
pub(crate) async fn save_recovery_file(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<bool, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || -> Result<bool, RuntimeError> {
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
}

fn u64_from_le_bytes(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("fixed u64 length"))
}
