use super::*;

const LOCAL_INBOX_WATCH_DEBOUNCE: Duration = Duration::from_millis(250);

impl VaultRuntime {
    pub(crate) fn configure_local_inbox(&self, root: &Path) -> Result<(), RuntimeError> {
        self.require_unlocked()?;
        let (bookmark, authorized_root) = authorize_root(root)
            .map_err(|_| RuntimeError::new("local_inbox_authorization_failed"))?;
        ensure_inbox_paths(authorized_root.root())
            .map_err(|_| RuntimeError::new("local_inbox_setup_failed"))?;
        self.inner
            .local_inbox_bookmarks
            .save(&bookmark)
            .map_err(|_| RuntimeError::new("local_inbox_storage_failed"))?;
        *self
            .inner
            .local_inbox_access
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))? = Some(authorized_root);
        self.inner
            .local_inbox_needs_reauthorization
            .store(false, Ordering::SeqCst);
        self.inner
            .local_inbox_needs_attention
            .store(false, Ordering::SeqCst);
        Ok(())
    }

    pub(crate) fn local_inbox_status(&self) -> Result<LocalInboxStatus, RuntimeError> {
        let configured = self
            .inner
            .local_inbox_bookmarks
            .load()
            .map_err(|_| RuntimeError::new("local_inbox_storage_failed"))?
            .is_some();
        let vault_status = self.status()?;
        if configured
            && vault_status == VaultStatus::Unlocked
            && !self
                .inner
                .local_inbox_access
                .lock()
                .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?
                .is_some()
        {
            self.activate_local_inbox_from_bookmark()?;
        }
        let access = self
            .inner
            .local_inbox_access
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?;
        let needs_reauthorization = self
            .inner
            .local_inbox_needs_reauthorization
            .load(Ordering::SeqCst);
        let needs_attention = self
            .inner
            .local_inbox_needs_attention
            .load(Ordering::SeqCst);
        let active = access.is_some();
        Ok(LocalInboxStatus {
            access_state: if !configured {
                LocalInboxAccessState::Disabled
            } else if needs_reauthorization {
                LocalInboxAccessState::NeedsReauthorization
            } else if needs_attention {
                LocalInboxAccessState::NeedsAttention
            } else if active {
                LocalInboxAccessState::Enabled
            } else {
                LocalInboxAccessState::Paused
            },
            backups_prepared: access
                .as_ref()
                .is_some_and(|root| root.root().join(BACKUPS_DIRECTORY_NAME).is_dir()),
            enabled: configured,
            inbox_label: "Inbox",
            last_scan: self
                .inner
                .local_inbox_last_scan
                .lock()
                .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?
                .clone(),
        })
    }

    pub(crate) fn disable_local_inbox(&self) -> Result<LocalInboxStatus, RuntimeError> {
        self.inner
            .local_inbox_bookmarks
            .delete()
            .map_err(|_| RuntimeError::new("local_inbox_storage_failed"))?;
        self.clear_local_inbox_access();
        self.inner
            .local_inbox_needs_reauthorization
            .store(false, Ordering::SeqCst);
        self.inner
            .local_inbox_needs_attention
            .store(false, Ordering::SeqCst);
        self.local_inbox_status()
    }

    pub(crate) fn list_statement_coverage_prompts(
        &self,
    ) -> Result<Vec<StatementCoveragePrompt>, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let today = store
            .statement_coverage_today()
            .map_err(|_| RuntimeError::new("coverage_unavailable"))?;
        store
            .list_statement_coverage_prompts(STATEMENT_COVERAGE_POLICIES, &today)
            .map_err(|_| RuntimeError::new("coverage_unavailable"))
    }

    pub(crate) fn record_statement_coverage_decision(
        &self,
        request: &StatementCoverageDecisionRequest,
    ) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let today = store
            .statement_coverage_today()
            .map_err(|_| RuntimeError::new("coverage_decision_invalid"))?;
        let is_current_prompt = store
            .list_statement_coverage_prompts(STATEMENT_COVERAGE_POLICIES, &today)
            .map_err(|_| RuntimeError::new("coverage_decision_invalid"))?
            .iter()
            .any(|prompt| {
                prompt.account_id == request.account_id
                    && prompt.document_type == request.document_type
                    && prompt.money_source_id == request.money_source_id
                    && prompt.statement_period_from == request.statement_period_from
                    && prompt.statement_period_to == request.statement_period_to
            });
        if !is_current_prompt {
            return Err(RuntimeError::new("coverage_decision_invalid"));
        }
        let decision = match request.action {
            StatementCoverageDecisionAction::NotExpected => StatementCoverageDecision::NotExpected,
            StatementCoverageDecisionAction::RemindLater => StatementCoverageDecision::RemindLater,
        };
        let audit_id = random_identifier("audit");
        store
            .record_statement_coverage_decision(&StatementCoverageDecisionInput {
                account_id: &request.account_id,
                audit_id: &audit_id,
                decision,
                document_type: &request.document_type,
                money_source_id: &request.money_source_id,
                remind_after: request.remind_after.as_deref(),
                statement_period_from: &request.statement_period_from,
                statement_period_to: &request.statement_period_to,
            })
            .map_err(|_| RuntimeError::new("coverage_decision_invalid"))
    }

    pub(crate) fn rescan_local_inbox(&self) -> Result<LocalInboxScanSummary, RuntimeError> {
        self.require_unlocked()?;
        if !self.activate_local_inbox_from_bookmark()? {
            let code = if self
                .inner
                .local_inbox_needs_reauthorization
                .load(Ordering::SeqCst)
            {
                "local_inbox_reauthorization_required"
            } else if self
                .inner
                .local_inbox_needs_attention
                .load(Ordering::SeqCst)
            {
                "local_inbox_setup_required"
            } else {
                "local_inbox_not_configured"
            };
            return Err(RuntimeError::new(code));
        }
        let access = self
            .inner
            .local_inbox_access
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?;
        let root = access
            .as_ref()
            .ok_or_else(|| RuntimeError::new("local_inbox_reauthorization_required"))?;
        let LocalInboxPaths { inbox, .. } = ensure_inbox_paths(root.root())
            .map_err(|_| RuntimeError::new("local_inbox_setup_failed"))?;
        let tombstoned_hashes = {
            let store = self.store()?;
            store
                .as_ref()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?
                .deleted_source_hashes()
                .map_err(|_| RuntimeError::new("local_inbox_scan_failed"))?
        };
        let entries =
            fs::read_dir(inbox).map_err(|_| RuntimeError::new("local_inbox_scan_failed"))?;
        let mut summary = LocalInboxScanSummary::default();
        let preflight = SystemNativePreflight;
        let mut candidates = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    summary.deferred += 1;
                    continue;
                }
            };
            let path = entry.path();
            match first_snapshot_after_preflight(&path, &preflight) {
                Ok(snapshot) => candidates.push((path, snapshot)),
                Err(_) => summary.deferred += 1,
            }
        }
        if !candidates.is_empty() {
            std::thread::sleep(SETTLE_INTERVAL);
        }
        for (path, first_snapshot) in candidates {
            if !preflight.permits_read(&path) {
                summary.deferred += 1;
                continue;
            }
            let mut persisted = None;
            let outcome =
                capture_after_second_scan(&path, first_snapshot, &tombstoned_hashes, |bytes| {
                    match self.register_local_inbox_capture(&path, bytes) {
                        Ok(outcome) => {
                            persisted = Some(outcome);
                            Ok(())
                        }
                        Err(_) => Err(io::Error::other("local Inbox import failed")),
                    }
                });
            match outcome {
                CaptureOutcome::Captured { .. } => match persisted {
                    Some(outcome) => match outcome.status {
                        SourceDocumentImportStatus::Imported
                        | SourceDocumentImportStatus::Restored => {
                            summary.imported += 1;
                        }
                        SourceDocumentImportStatus::AlreadyPresent => summary.already_present += 1,
                        SourceDocumentImportStatus::RestoreConfirmationRequired => {
                            summary.suppressed += 1;
                        }
                    },
                    None => summary.deferred += 1,
                },
                CaptureOutcome::Suppressed { .. } => summary.suppressed += 1,
                CaptureOutcome::Deferred(_) => summary.deferred += 1,
            }
        }
        *self
            .inner
            .local_inbox_last_scan
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))? = Some(summary.clone());
        Ok(summary)
    }

    fn start_local_inbox_watcher(&self, app: &AppHandle) -> Result<(), RuntimeError> {
        let inbox = {
            let access = self
                .inner
                .local_inbox_access
                .lock()
                .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?;
            let root = access
                .as_ref()
                .ok_or_else(|| RuntimeError::new("local_inbox_reauthorization_required"))?;
            ensure_inbox_paths(root.root())
                .map_err(|_| RuntimeError::new("local_inbox_setup_failed"))?
                .inbox
        };
        let app = app.clone();
        let inner = Arc::downgrade(&self.inner);
        let watcher = LocalInboxWatcher::start(&inbox, move || {
            schedule_local_inbox_rescan(app.clone(), inner.clone());
        })
        .map_err(|_| {
            self.inner
                .local_inbox_needs_attention
                .store(true, Ordering::SeqCst);
            RuntimeError::new("local_inbox_watch_failed")
        })?;
        *self
            .inner
            .local_inbox_watcher
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))? = Some(watcher);
        Ok(())
    }

    pub(super) fn register_local_inbox_capture(
        &self,
        source_path: &Path,
        captured_bytes: Zeroizing<Vec<u8>>,
    ) -> Result<SourceDocumentImportOutcome, RuntimeError> {
        let (original_filename, mime_type) = source_document_filename_metadata(source_path)?;
        let document_id = random_identifier("document");
        let audit_id = random_identifier("audit");
        let input = SourceDocumentImport {
            audit_actor: "system",
            audit_id: &audit_id,
            audit_policy_version: IMPORT_POLICY_VERSION,
            audit_reason: "local_inbox_import",
            document_id: &document_id,
            mime_type,
            original_filename: &original_filename,
            source_path,
        };
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let outcome = store
            .register_captured_import(&input, captured_bytes, None)
            .map_err(|_| RuntimeError::new("local_inbox_import_failed"))?;
        if outcome.status != SourceDocumentImportStatus::RestoreConfirmationRequired {
            store
                .enqueue_source_document_pipeline(&outcome.document_id)
                .map_err(|_| RuntimeError::new("local_inbox_import_failed"))?;
        }
        Ok(outcome)
    }

    pub(super) fn queued_local_inbox_parse_documents(&self) -> Result<Vec<String>, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .queued_parse_document_ids()
            .map_err(|_| RuntimeError::new("local_inbox_parse_failed"))
    }

    pub(super) fn start_local_inbox_parse(&self, document_id: &str) -> Result<bool, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .start_parse_document(document_id)
            .map_err(|_| RuntimeError::new("local_inbox_parse_failed"))
    }

    pub(super) fn block_local_inbox_parse(
        &self,
        document_id: &str,
        reason: &'static str,
    ) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .block_parse_document(document_id, reason)
            .map_err(|_| RuntimeError::new("local_inbox_parse_failed"))
    }

    pub(super) fn fail_local_inbox_parse(
        &self,
        document_id: &str,
        reason: &'static str,
    ) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .fail_parse_document(document_id, reason)
            .map_err(|_| RuntimeError::new("local_inbox_parse_failed"))
    }

    pub(super) fn queued_document_reconciliations(&self) -> Result<Vec<String>, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .queued_reconcile_document_ids()
            .map_err(|_| RuntimeError::new("reconcile_failed"))
    }

    pub(super) fn start_document_reconciliation(
        &self,
        document_id: &str,
    ) -> Result<bool, RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .start_reconcile_document(document_id)
            .map_err(|_| RuntimeError::new("reconcile_failed"))
    }

    pub(super) fn reconcile_document(&self, document_id: &str) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .reconcile_document(document_id)
            .map_err(|_| RuntimeError::new("reconcile_failed"))
    }

    pub(super) fn fail_document_reconciliation(
        &self,
        document_id: &str,
        reason: &'static str,
    ) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .fail_reconcile_document(document_id, reason)
            .map_err(|_| RuntimeError::new("reconcile_failed"))
    }

    pub(super) fn activate_local_inbox_from_bookmark(&self) -> Result<bool, RuntimeError> {
        if self
            .inner
            .local_inbox_access
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?
            .is_some()
        {
            return Ok(true);
        }
        let Some(bookmark) = self
            .inner
            .local_inbox_bookmarks
            .load()
            .map_err(|_| RuntimeError::new("local_inbox_storage_failed"))?
        else {
            return Ok(false);
        };
        match resolve_root_bookmark(&bookmark) {
            Ok(BookmarkResolution::Active(root)) => {
                if ensure_inbox_paths(root.root()).is_err() {
                    self.inner
                        .local_inbox_needs_attention
                        .store(true, Ordering::SeqCst);
                    return Ok(false);
                }
                *self
                    .inner
                    .local_inbox_access
                    .lock()
                    .map_err(|_| RuntimeError::new("local_inbox_unavailable"))? = Some(root);
                self.inner
                    .local_inbox_needs_reauthorization
                    .store(false, Ordering::SeqCst);
                self.inner
                    .local_inbox_needs_attention
                    .store(false, Ordering::SeqCst);
                Ok(true)
            }
            Ok(BookmarkResolution::Stale) | Err(_) => {
                self.inner
                    .local_inbox_needs_reauthorization
                    .store(true, Ordering::SeqCst);
                self.inner
                    .local_inbox_needs_attention
                    .store(false, Ordering::SeqCst);
                Ok(false)
            }
        }
    }

    pub(super) fn clear_local_inbox_access(&self) {
        if let Ok(mut watcher) = self.inner.local_inbox_watcher.lock() {
            *watcher = None;
        }
        if let Ok(mut access) = self.inner.local_inbox_access.lock() {
            *access = None;
        }
    }
}

fn schedule_local_inbox_rescan(app: AppHandle, inner: Weak<RuntimeInner>) {
    let Some(inner) = inner.upgrade() else {
        return;
    };
    inner.local_inbox_scan_pending.store(true, Ordering::SeqCst);
    if inner
        .local_inbox_scan_scheduled
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let runtime = VaultRuntime { inner };
        loop {
            sleep(LOCAL_INBOX_WATCH_DEBOUNCE).await;
            runtime
                .inner
                .local_inbox_scan_pending
                .store(false, Ordering::SeqCst);
            let _ = rescan_and_process_local_inbox(&app, runtime.clone()).await;
            if runtime
                .inner
                .local_inbox_scan_pending
                .swap(false, Ordering::SeqCst)
            {
                continue;
            }
            runtime
                .inner
                .local_inbox_scan_scheduled
                .store(false, Ordering::SeqCst);
            if runtime
                .inner
                .local_inbox_scan_pending
                .swap(false, Ordering::SeqCst)
                && runtime
                    .inner
                    .local_inbox_scan_scheduled
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
            {
                continue;
            }
            break;
        }
    });
}

pub(super) async fn rescan_and_process_local_inbox(
    app: &AppHandle,
    runtime: VaultRuntime,
) -> Result<LocalInboxScanSummary, VaultCommandError> {
    let summary = {
        let runtime = runtime.clone();
        tauri::async_runtime::spawn_blocking(move || runtime.rescan_local_inbox())
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    process_queued_local_inbox_parses(app, runtime).await?;
    Ok(summary)
}

pub(super) async fn process_queued_local_inbox_parses(
    app: &AppHandle,
    runtime: VaultRuntime,
) -> Result<(), VaultCommandError> {
    let document_ids = {
        let runtime = runtime.clone();
        tauri::async_runtime::spawn_blocking(move || runtime.queued_local_inbox_parse_documents())
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    for document_id in document_ids {
        let started = {
            let runtime = runtime.clone();
            let document_id = document_id.clone();
            tauri::async_runtime::spawn_blocking(move || {
                runtime.start_local_inbox_parse(&document_id)
            })
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
        };
        if !started {
            continue;
        }
        let input = {
            let runtime = runtime.clone();
            let document_id = document_id.clone();
            tauri::async_runtime::spawn_blocking(move || runtime.normalization_input(&document_id))
                .await
                .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        };
        let input = match input {
            Ok(input) => input,
            Err(error) => {
                let reason = if error.code == "statement_password_required" {
                    "password_required"
                } else {
                    "parse_input_unavailable"
                };
                let runtime = runtime.clone();
                let document_id = document_id.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    runtime.block_local_inbox_parse(&document_id, reason)
                })
                .await
                .map_err(|_| VaultCommandError::new("runtime_unavailable"))??;
                continue;
            }
        };
        let result = match run_normalizer_sidecar(app, &document_id, &input).await {
            Ok(result) => result,
            Err(_) => {
                let runtime = runtime.clone();
                let document_id = document_id.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    runtime.fail_local_inbox_parse(&document_id, "normalizer_failed")
                })
                .await
                .map_err(|_| VaultCommandError::new("runtime_unavailable"))??;
                continue;
            }
        };
        let applied = {
            let runtime = runtime.clone();
            let document_id = document_id.clone();
            tauri::async_runtime::spawn_blocking(move || {
                runtime.apply_normalizer_result(&document_id, &input, result)
            })
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        };
        if applied.is_err() {
            let runtime = runtime.clone();
            let document_id = document_id.clone();
            tauri::async_runtime::spawn_blocking(move || {
                runtime.fail_local_inbox_parse(&document_id, "classification_failed")
            })
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??;
        }
    }
    process_queued_document_reconciliations(runtime).await?;
    Ok(())
}

pub(super) async fn process_queued_document_reconciliations(
    runtime: VaultRuntime,
) -> Result<(), VaultCommandError> {
    let document_ids = {
        let runtime = runtime.clone();
        tauri::async_runtime::spawn_blocking(move || runtime.queued_document_reconciliations())
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
    };
    for document_id in document_ids {
        let started = {
            let runtime = runtime.clone();
            let document_id = document_id.clone();
            tauri::async_runtime::spawn_blocking(move || {
                runtime.start_document_reconciliation(&document_id)
            })
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
        };
        if !started {
            continue;
        }
        let reconciled = {
            let runtime = runtime.clone();
            let document_id = document_id.clone();
            tauri::async_runtime::spawn_blocking(move || runtime.reconcile_document(&document_id))
                .await
                .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
        };
        if reconciled.is_err() {
            let runtime = runtime.clone();
            tauri::async_runtime::spawn_blocking(move || {
                runtime.fail_document_reconciliation(&document_id, "reconcile_failed")
            })
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??;
        }
    }
    Ok(())
}

pub(super) async fn resume_local_inbox_after_unlock(app: &AppHandle, runtime: VaultRuntime) {
    let status = tauri::async_runtime::spawn_blocking({
        let runtime = runtime.clone();
        move || runtime.local_inbox_status()
    })
    .await
    .ok()
    .and_then(Result::ok);
    if status.is_some_and(|status| status.access_state == LocalInboxAccessState::Enabled) {
        let _ = runtime.start_local_inbox_watcher(app);
        let _ = rescan_and_process_local_inbox(app, runtime).await;
    }
}

pub(super) async fn resume_document_reconciliations_after_unlock(runtime: VaultRuntime) {
    let _ = process_queued_document_reconciliations(runtime).await;
}

#[tauri::command]
pub(crate) async fn choose_local_inbox_root(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<Option<LocalInboxStatus>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let configure_runtime = runtime.clone();
    let picker_app = app.clone();
    let configured = tauri::async_runtime::spawn_blocking(move || -> Result<bool, RuntimeError> {
        configure_runtime.require_unlocked()?;
        let selected = picker_app
            .dialog()
            .file()
            .set_title("Choose your Cancan folder")
            .blocking_pick_folder();
        let Some(selected) = selected else {
            return Ok(false);
        };
        let path = selected
            .into_path()
            .map_err(|_| RuntimeError::new("file_selection_failed"))?;
        configure_runtime.configure_local_inbox(&path)?;
        Ok(true)
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(VaultCommandError::from)?;
    if !configured {
        return Ok(None);
    }
    runtime
        .start_local_inbox_watcher(&app)
        .map_err(VaultCommandError::from)?;
    rescan_and_process_local_inbox(&app, runtime.clone()).await?;
    run_runtime_task(move || runtime.local_inbox_status().map(Some)).await
}

#[tauri::command]
pub(crate) async fn local_inbox_status(
    runtime: State<'_, VaultRuntime>,
) -> Result<LocalInboxStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.local_inbox_status()).await
}

#[tauri::command]
pub(crate) async fn disable_local_inbox(
    runtime: State<'_, VaultRuntime>,
) -> Result<LocalInboxStatus, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.disable_local_inbox()).await
}

#[tauri::command]
pub(crate) async fn list_statement_coverage_prompts(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<StatementCoveragePrompt>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_statement_coverage_prompts()).await
}

#[tauri::command]
pub(crate) async fn record_statement_coverage_decision(
    request: StatementCoverageDecisionRequest,
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.record_statement_coverage_decision(&request)).await
}

#[tauri::command]
pub(crate) async fn rescan_local_inbox(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<LocalInboxScanSummary, VaultCommandError> {
    let runtime = runtime.inner().clone();
    rescan_and_process_local_inbox(&app, runtime).await
}
