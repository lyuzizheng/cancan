use super::*;

#[derive(Clone)]
pub(super) struct ParseDocumentAttempt {
    pub(super) claim: ParseDocumentClaim,
    pub(super) vault_session_generation: u64,
}

impl VaultRuntime {
    pub(super) fn mark_local_inbox_needs_attention(&self) {
        self.inner
            .local_inbox_needs_attention
            .store(true, Ordering::SeqCst);
    }

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

    /// Requeues jobs whose lease expired so this pump pass can retry them.
    /// Recovery is a write, so it belongs here rather than in the queue reads.
    pub(super) fn recover_expired_jobs(&self) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .recover_expired_jobs()
            .map_err(|_| RuntimeError::new("job_recovery_failed"))
    }

    pub(super) fn queued_local_inbox_parse_documents(
        &self,
    ) -> Result<Vec<ParseDocumentJob>, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .queued_parse_document_jobs()
            .map_err(|_| RuntimeError::new("local_inbox_parse_failed"))
    }

    pub(super) fn enqueue_source_document_reparse(
        &self,
        document_id: &str,
    ) -> Result<(), RuntimeError> {
        if document_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        let mut store = self.store()?;
        let enqueued = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .enqueue_source_document_pipeline(document_id)
            .map_err(|_| RuntimeError::new("local_inbox_parse_failed"))?;
        if !enqueued {
            return Err(RuntimeError::new("parse_already_running"));
        }
        Ok(())
    }

    pub(super) fn start_local_inbox_parse(
        &self,
        job: &ParseDocumentJob,
    ) -> Result<Option<ParseDocumentAttempt>, RuntimeError> {
        let vault_session_generation = self.inner.vault_session_generation.load(Ordering::SeqCst);
        self.require_vault_session(vault_session_generation)?;
        let mut store = self.store()?;
        let claim = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .start_parse_document_job(job)
            .map_err(|_| RuntimeError::new("local_inbox_parse_failed"))?;
        drop(store);
        self.require_vault_session(vault_session_generation)?;
        Ok(claim.map(|claim| ParseDocumentAttempt {
            claim,
            vault_session_generation,
        }))
    }

    pub(super) fn block_local_inbox_parse(
        &self,
        attempt: &ParseDocumentAttempt,
        reason: &'static str,
    ) -> Result<(), RuntimeError> {
        self.require_vault_session(attempt.vault_session_generation)?;
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .block_parse_document_job(&attempt.claim, reason)
            .map_err(|_| RuntimeError::new("local_inbox_parse_failed"))
    }

    pub(super) fn fail_local_inbox_parse(
        &self,
        attempt: &ParseDocumentAttempt,
        reason: &'static str,
    ) -> Result<(), RuntimeError> {
        self.require_vault_session(attempt.vault_session_generation)?;
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .fail_parse_document_job(&attempt.claim, reason)
            .map_err(|_| RuntimeError::new("local_inbox_parse_failed"))
    }

    pub(super) fn queued_document_reconciliations(&self) -> Result<Vec<String>, RuntimeError> {
        let store = self.store()?;
        store
            .as_ref()
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
    let _ = runtime.start_local_inbox_watcher(app.clone());
    schedule_queued_local_inbox_parses(app.clone(), runtime);
    Ok(summary)
}

pub(super) fn schedule_queued_local_inbox_parses(app: AppHandle, runtime: VaultRuntime) {
    tauri::async_runtime::spawn(async move {
        let _ = process_queued_local_inbox_parses(&app, runtime).await;
    });
}

pub(super) async fn process_queued_local_inbox_parses(
    app: &AppHandle,
    runtime: VaultRuntime,
) -> Result<(), VaultCommandError> {
    tauri::async_runtime::spawn_blocking({
        let runtime = runtime.clone();
        move || runtime.recover_expired_jobs()
    })
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))??;
    loop {
        let document_ids = {
            let runtime = runtime.clone();
            tauri::async_runtime::spawn_blocking(move || {
                runtime.queued_local_inbox_parse_documents()
            })
            .await
            .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
        };
        if document_ids.is_empty() {
            break;
        }
        for job in document_ids {
            let attempt = {
                let runtime = runtime.clone();
                let job = job.clone();
                tauri::async_runtime::spawn_blocking(move || runtime.start_local_inbox_parse(&job))
                    .await
                    .map_err(|_| VaultCommandError::new("runtime_unavailable"))??
            };
            let Some(attempt) = attempt else {
                continue;
            };
            let input = {
                let runtime = runtime.clone();
                let document_id = job.document_id.clone();
                let vault_session_generation = attempt.vault_session_generation;
                tauri::async_runtime::spawn_blocking(move || {
                    runtime.normalization_input_for_vault_session(
                        &document_id,
                        vault_session_generation,
                    )
                })
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
                    let attempt = attempt.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        runtime.block_local_inbox_parse(&attempt, reason)
                    })
                    .await
                    .map_err(|_| VaultCommandError::new("runtime_unavailable"))??;
                    continue;
                }
            };
            let result = match run_normalizer_sidecar(app, &job.document_id, &input.bundle).await {
                Ok(result) => result,
                Err(_) => {
                    let runtime = runtime.clone();
                    let attempt = attempt.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        runtime.fail_local_inbox_parse(&attempt, "normalizer_failed")
                    })
                    .await
                    .map_err(|_| VaultCommandError::new("runtime_unavailable"))??;
                    continue;
                }
            };
            let applied = {
                let runtime = runtime.clone();
                let claim = attempt.claim.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    runtime.apply_normalizer_result_for_job(&claim, &input, result)
                })
                .await
                .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
            };
            if applied.is_err() {
                let runtime = runtime.clone();
                let attempt = attempt.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    runtime.block_local_inbox_parse(&attempt, "classification_failed")
                })
                .await
                .map_err(|_| VaultCommandError::new("runtime_unavailable"))??;
            }
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
    // The pipeline just advanced, so this is the write side of the Tasks
    // projection: batches whose items reached a terminal state complete here.
    // The Tasks read command itself stays a pure read.
    tauri::async_runtime::spawn_blocking(move || runtime.seal_completed_intake_batches())
        .await
        .map_err(|_| VaultCommandError::new("runtime_unavailable"))??;
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
        let _ = runtime.start_local_inbox_watcher(app.clone());
        let _ = rescan_and_process_local_inbox(app, runtime).await;
    }
}

pub(super) async fn resume_parse_document_jobs_after_unlock(
    app: &AppHandle,
    runtime: VaultRuntime,
) {
    schedule_queued_local_inbox_parses(app.clone(), runtime);
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
        .start_local_inbox_watcher(app.clone())
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
pub(crate) async fn rescan_local_inbox(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<LocalInboxScanSummary, VaultCommandError> {
    let runtime = runtime.inner().clone();
    rescan_and_process_local_inbox(&app, runtime).await
}

#[tauri::command]
pub(crate) async fn reparse_source_document(
    document_id: String,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    if document_id.is_empty() {
        return Err(VaultCommandError::new("invalid_document_request"));
    }
    let runtime = runtime.inner().clone();
    let enqueue_runtime = runtime.clone();
    run_runtime_task(move || enqueue_runtime.enqueue_source_document_reparse(&document_id)).await?;
    schedule_queued_local_inbox_parses(app, runtime);
    Ok(())
}
