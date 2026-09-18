use super::*;

impl VaultRuntime {
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
        if result.is_ok() {
            self.document_passwords()?.remove(document_id);
            self.forget_cached_source_document(document_id);
        }
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
            .map_store_error(store, "source_document_mime_type", "document_unavailable")?;
        let generation = self.inner.vault_session_generation.load(Ordering::SeqCst);
        Ok((mime_type, generation))
    }

    fn source_document_read_plan(
        &self,
        document_id: &str,
    ) -> Result<(SourceDocumentReadPlan, u64), RuntimeError> {
        let generation = self.inner.vault_session_generation.load(Ordering::SeqCst);
        let plan = {
            let store = self.store()?;
            let store = store
                .as_ref()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            store
                .source_document_read_plan(document_id)
                .map_store_error(store, "source_document_read_plan", "document_unavailable")?
        };
        Ok((plan, generation))
    }

    fn read_source_document(
        &self,
        document_id: &str,
    ) -> Result<SourceDocumentFileInput, RuntimeError> {
        let (plan, generation) = self.source_document_read_plan(document_id)?;
        if let Some(cached) =
            self.cached_source_document(document_id, generation, plan.file_sha256())
        {
            return Ok(cached);
        }
        let input = plan.read().map_err(|error| {
            runtime_failure(
                self,
                "read_source_document",
                "document_unavailable",
                &*error,
            )
        })?;
        self.require_vault_session(generation)?;
        self.remember_source_document(document_id, generation, &input)?;
        Ok(input)
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
        ensure_copy_outside_vault(self, &self.inner.root, destination)?;
        self.require_vault_session(session_generation)?;
        let input = self.read_source_document(document_id)?;
        self.require_vault_session(session_generation)?;
        write_export_atomically(destination, &input.plaintext).map_err(|error| {
            runtime_failure(
                self,
                "write_export_atomically",
                "source_copy_save_failed",
                &error,
            )
        })
    }

    pub(super) fn require_unlocked(&self) -> Result<(), RuntimeError> {
        if self.store()?.is_none() {
            return Err(RuntimeError::new("vault_locked"));
        }
        Ok(())
    }

    pub(super) fn require_vault_session(&self, generation: u64) -> Result<(), RuntimeError> {
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != generation {
            return Err(RuntimeError::new("vault_locked"));
        }
        self.require_unlocked()
    }

    pub(super) fn advance_vault_session(&self) {
        self.inner
            .vault_session_generation
            .fetch_add(1, Ordering::SeqCst);
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
        let documents = store.list_documents(money_source_id).map_store_error(
            store,
            "list_documents",
            "list_documents_failed",
        )?;
        Self::source_document_summaries(store, documents)
    }

    pub(crate) fn list_money_sources(&self) -> Result<Vec<MoneySourceSummary>, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .list_money_sources()
            .map(|sources| {
                sources
                    .into_iter()
                    .map(|source| MoneySourceSummary {
                        display_name: source.display_name,
                        money_source_id: source.money_source_id,
                        source_type: source.source_type,
                    })
                    .collect()
            })
            .map_err(|error| {
                store_failure(store, "list_money_sources", "list_sources_failed", &*error)
            })
    }

    pub(crate) fn list_account_confirmation_prompts(
        &self,
    ) -> Result<Vec<AccountConfirmationPrompt>, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store.list_account_confirmation_prompts().map_store_error(
            store,
            "list_account_confirmation_prompts",
            "account_confirmation_unavailable",
        )
    }

    pub(crate) fn decide_candidate_accounts(
        &self,
        money_source_id: &str,
        proposal_version: &str,
        decisions: &[CandidateAccountDecisionInput],
    ) -> Result<AccountConfirmationOutcome, RuntimeError> {
        if money_source_id.is_empty() || proposal_version.is_empty() || decisions.is_empty() {
            return Err(RuntimeError::new("invalid_account_confirmation_request"));
        }
        let audit_id = random_identifier("audit");
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .decide_candidate_accounts(money_source_id, proposal_version, decisions, &audit_id)
            .map_store_error(
                store,
                "decide_candidate_accounts",
                "account_confirmation_unavailable",
            )
    }

    pub(crate) fn restore_dismissed_candidate_account(
        &self,
        account_id: &str,
    ) -> Result<AccountConfirmationOutcome, RuntimeError> {
        if account_id.is_empty() {
            return Err(RuntimeError::new("invalid_account_confirmation_request"));
        }
        let audit_id = random_identifier("audit");
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .restore_dismissed_candidate_account(account_id, &audit_id)
            .map_store_error(
                store,
                "restore_dismissed_candidate_account",
                "account_confirmation_unavailable",
            )
    }

    pub(crate) fn list_unassigned_source_documents(
        &self,
    ) -> Result<Vec<SourceDocumentSummary>, RuntimeError> {
        let store = self.store()?;
        let store = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        let documents = store.list_unassigned_documents().map_store_error(
            store,
            "list_unassigned_documents",
            "list_documents_failed",
        )?;
        Self::source_document_summaries(store, documents)
    }

    fn source_document_summaries(
        store: &ManualImportStore,
        documents: Vec<SourceDocumentView>,
    ) -> Result<Vec<SourceDocumentSummary>, RuntimeError> {
        let document_ids = documents
            .iter()
            .map(|document| document.document_id.clone())
            .collect::<Vec<_>>();
        let mut parse_statuses = store
            .source_document_parse_statuses(&document_ids)
            .map_store_error(
                store,
                "source_document_parse_statuses",
                "list_documents_failed",
            )?;
        Ok(documents
            .into_iter()
            .map(|document| {
                let parse_status = parse_statuses.remove(&document.document_id);
                source_document_summary(document, parse_status)
            })
            .collect())
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
            .map_err(|error| {
                store_failure(
                    store,
                    "statement_password_sources",
                    "list_sources_failed",
                    &*error,
                )
            })
    }

    /// Host-owned bounded pass over the Vault's distinct saved statement
    /// passwords.
    ///
    /// This is the pass for a protected statement whose Money Source is not
    /// yet known: the renderer neither selects the attempted secret nor learns
    /// which Money Source, if any, holds the one that worked. Each distinct
    /// saved password is probed at most once and only the outcome is reported.
    /// A successful password decrypts this document's bytes for the current
    /// Vault session and keeps the Keychain entry it came from — it never
    /// assigns, moves, or infers the document's Money Source.
    pub(crate) fn try_saved_statement_passwords(
        &self,
        document_id: &str,
    ) -> Result<SavedStatementPasswordResult, RuntimeError> {
        if document_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        let candidates = {
            let store = self.store()?;
            let store = store
                .as_ref()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            self.reconcile_statement_passwords(store)?;
            store.saved_statement_password_states().map_store_error(
                store,
                "saved_statement_password_states",
                "list_sources_failed",
            )?
        };
        if candidates.is_empty() {
            return Ok(SavedStatementPasswordResult::Unavailable);
        }
        // A malformed request fails closed before any saved secret is loaded.
        let input = self.read_source_document(document_id)?;
        if input.mime_type != "application/pdf" {
            return Err(RuntimeError::new("viewer_unsupported"));
        }
        let mut attempted: Vec<Zeroizing<Vec<u8>>> = Vec::with_capacity(candidates.len());
        let mut any_loaded = false;
        for state in candidates {
            let Some(password) = self
                .inner
                .statement_passwords
                .load(&state.secret_storage_key)
                .map_err(|error| {
                    runtime_failure(
                        self,
                        "statement_password_load",
                        "statement_password_load_failed",
                        &error,
                    )
                })?
            else {
                continue;
            };
            any_loaded = true;
            // Distinct secrets only: a password another source already
            // contributed to this attempt is never probed twice.
            if attempted
                .iter()
                .any(|prior| prior.as_slice() == password.as_slice())
            {
                continue;
            }
            #[cfg(test)]
            self.inner
                .statement_password_attempts
                .fetch_add(1, Ordering::SeqCst);
            if statement_password_unlocks(&input, &password)? {
                self.document_passwords()?
                    .insert(document_id.to_owned(), password);
                self.resume_password_blocked_parse_document_job(document_id)?;
                return Ok(SavedStatementPasswordResult::Unlocked);
            }
            attempted.push(password);
        }
        Ok(if any_loaded {
            SavedStatementPasswordResult::Invalid
        } else {
            SavedStatementPasswordResult::Unavailable
        })
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
        {
            let store = self.store()?;
            let store = store
                .as_ref()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            ensure_statement_password_source(store, money_source_id)?;
        }
        let input = self.read_source_document(document_id)?;
        if !statement_password_unlocks(&input, password)? {
            return Err(RuntimeError::new("statement_password_invalid"));
        }
        if update_saved_password {
            let store = self.store()?;
            let store = store
                .as_ref()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            self.save_statement_password_in_store(store, money_source_id, password)?;
        }
        self.document_passwords()?
            .insert(document_id.to_owned(), Zeroizing::new(password.to_vec()));
        self.resume_password_blocked_parse_document_job(document_id)?;
        Ok(())
    }

    fn resume_password_blocked_parse_document_job(
        &self,
        document_id: &str,
    ) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .requeue_password_blocked_parse_document_job(document_id)
            .map_store_error(
                store,
                "requeue_password_blocked_parse_document_job",
                "parse_resume_failed",
            )?;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn normalization_input(
        &self,
        document_id: &str,
    ) -> Result<ExtractedDocument, RuntimeError> {
        let vault_session_generation = self.inner.vault_session_generation.load(Ordering::SeqCst);
        self.normalization_input_for_vault_session(document_id, vault_session_generation)
    }

    pub(super) fn normalization_input_for_vault_session(
        &self,
        document_id: &str,
        vault_session_generation: u64,
    ) -> Result<ExtractedDocument, RuntimeError> {
        self.require_vault_session(vault_session_generation)?;
        let input = self.read_source_document(document_id)?;
        let password = if input.mime_type == "application/pdf" {
            let passwords = self.document_passwords()?;
            passwords.get(document_id).cloned()
        } else {
            None
        };
        let bundle = extract_bundle(
            document_id,
            &input.file_sha256,
            &input.mime_type,
            &input.plaintext,
            password.as_ref().map(|value| value.as_slice()),
        )
        .map_err(|error| {
            if input.mime_type == "application/pdf" {
                document_render_error(error)
            } else {
                RuntimeError::new("normalizer_failed")
            }
        })?;
        self.require_vault_session(vault_session_generation)?;
        Ok(ExtractedDocument {
            bundle,
            vault_session_generation,
        })
    }

    pub(super) fn render_source_document_page(
        &self,
        document_id: &str,
        page_number: u32,
    ) -> Result<RenderedDocumentPage, RuntimeError> {
        if document_id.is_empty() || page_number == 0 {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        let input = self.read_source_document(document_id)?;
        match input.mime_type.as_str() {
            "application/pdf" => {
                let passwords = self.document_passwords()?;
                let password = passwords.get(document_id).map(|value| value.as_slice());
                render_pdf_page_with_password(&input.plaintext, page_number, password)
                    .map_err(document_render_error)
            }
            "image/png" | "image/jpeg" => {
                render_image_document(&input.plaintext, &input.mime_type, page_number)
                    .map_err(document_render_error)
            }
            _ => Err(RuntimeError::new("viewer_unsupported")),
        }
    }

    pub(super) fn preview_source_document(
        &self,
        document_id: &str,
    ) -> Result<SourceDocumentPreview, RuntimeError> {
        if document_id.is_empty() {
            return Err(RuntimeError::new("invalid_document_request"));
        }
        let input = self.read_source_document(document_id)?;
        if input.mime_type != "text/csv" {
            return Err(RuntimeError::new("viewer_unsupported"));
        }
        Ok(bounded_text_preview(&input.plaintext))
    }
}

/// One listed document plus the parse status its caller batched for the list.
fn source_document_summary(
    document: SourceDocumentView,
    parse_status: Option<(String, Option<String>)>,
) -> SourceDocumentSummary {
    let (document_status, attention_reason) = if document.file_state == "deleted" {
        (SourceDocumentStatus::FileDeleted, None)
    } else if document.file_state == "missing" {
        (SourceDocumentStatus::Missing, None)
    } else if parse_status
        .as_ref()
        .is_some_and(|(status, _)| matches!(status.as_str(), "queued" | "running"))
    {
        (SourceDocumentStatus::Processing, None)
    } else if let Some((status, reason)) =
        parse_status.filter(|(status, _)| matches!(status.as_str(), "blocked" | "failed"))
    {
        (
            SourceDocumentStatus::NeedsAttention,
            reason.or(Some(status)),
        )
    } else {
        (SourceDocumentStatus::Ready, None)
    };
    SourceDocumentSummary {
        attention_reason,
        byte_size: document.byte_size,
        document_status,
        document_id: document.document_id,
        file_state: document.file_state,
        mime_type: document.mime_type,
        original_filename: document.original_filename,
        received_at: document.received_at,
    }
}

pub(super) fn document_render_error(error: io::Error) -> RuntimeError {
    match error.kind() {
        io::ErrorKind::InvalidInput => RuntimeError::new("invalid_document_request"),
        io::ErrorKind::PermissionDenied => RuntimeError::new("statement_password_required"),
        io::ErrorKind::Unsupported => RuntimeError::new("viewer_unsupported"),
        _ => RuntimeError::new("document_render_failed"),
    }
}

pub(super) fn bounded_text_preview(plaintext: &[u8]) -> SourceDocumentPreview {
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
        let separator = usize::from(preview_lines > 0);
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

pub(super) fn statement_password_unlocks(
    input: &crate::database::SourceDocumentFileInput,
    password: &[u8],
) -> Result<bool, RuntimeError> {
    if input.mime_type != "application/pdf" {
        return Err(RuntimeError::new("viewer_unsupported"));
    }
    // One parse: the probe reports both whether the document is protected at
    // all and whether this password unlocks it.
    match pdf_password_unlocks(&input.plaintext, password).map_err(document_render_error)? {
        Some(unlocked) => Ok(unlocked),
        None => Err(RuntimeError::new("document_not_protected")),
    }
}

pub(super) fn ensure_statement_password_source(
    store: &ManualImportStore,
    money_source_id: &str,
) -> Result<(), RuntimeError> {
    if store.money_source_exists(money_source_id).map_store_error(
        store,
        "money_source_exists",
        "invalid_source_request",
    )? {
        Ok(())
    } else {
        Err(RuntimeError::new("invalid_source_request"))
    }
}

#[tauri::command]
pub(crate) async fn remove_statement_password(
    money_source_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.remove_statement_password(&money_source_id)).await
}

#[tauri::command]
pub(crate) async fn list_statement_password_sources(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<StatementPasswordSourceSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.statement_password_sources()).await
}

#[tauri::command]
pub(crate) async fn try_saved_statement_passwords(
    document_id: String,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<SavedStatementPasswordResult, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let try_runtime = runtime.clone();
    let result =
        run_runtime_task(move || try_runtime.try_saved_statement_passwords(&document_id)).await?;
    if result == SavedStatementPasswordResult::Unlocked {
        schedule_queued_local_inbox_parses(app, runtime);
    }
    Ok(result)
}

#[tauri::command]
pub(crate) async fn unlock_source_document(
    document_id: String,
    money_source_id: String,
    password: String,
    update_saved_password: bool,
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<(), VaultCommandError> {
    let runtime = runtime.inner().clone();
    let unlock_runtime = runtime.clone();
    let password = Zeroizing::new(password);
    run_runtime_task(move || {
        unlock_runtime.unlock_source_document(
            &document_id,
            &money_source_id,
            password.as_bytes(),
            update_saved_password,
        )
    })
    .await?;
    schedule_queued_local_inbox_parses(app, runtime);
    Ok(())
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
    run_runtime_task(move || -> Result<bool, RuntimeError> {
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
            "image/png" => "CanCan source copy.png",
            "image/jpeg" => "CanCan source copy.jpg",
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
        let destination = selected.into_path().map_err(|error| {
            runtime_failure(&runtime, "save_a_copy_destination", "file_selection_failed", &error)
        })?;
        runtime.save_source_document_copy(&document_id, &destination, session_generation)?;
        Ok(true)
    })
    .await
}

#[tauri::command]
pub(crate) async fn import_source_document(
    app: AppHandle,
    runtime: State<'_, VaultRuntime>,
) -> Result<Option<SourceDocumentImportOutcome>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    let import_runtime = runtime.clone();
    let import_app = app.clone();
    let outcome = tauri::async_runtime::spawn_blocking(
        move || -> Result<Option<SourceDocumentImportOutcome>, RuntimeError> {
        import_runtime.require_unlocked()?;
        let selected = import_app
            .dialog()
            .file()
            .set_title("Import a statement")
            .add_filter("Financial documents", &["pdf", "csv", "png", "jpg", "jpeg"])
            .blocking_pick_file();
        let Some(selected) = selected else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|error| {
            runtime_failure(
                &import_runtime,
                "import_destination",
                "file_selection_failed",
                &error,
            )
        })?;
        let outcome = import_runtime.import_selected_document(&path)?;
        if outcome.status != SourceDocumentImportStatus::RestoreConfirmationRequired {
            return Ok(Some(outcome));
        }
        let restore = import_app
            .dialog()
            .message(
                "This exact file was previously deleted from CanCan's Vault. Restore it to the existing document entry?",
            )
            .title("Restore source file?")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Restore file".to_owned(),
                "Leave deleted".to_owned(),
            ))
            .blocking_show();
        let Some(intake_item_id) = outcome.intake_item_id.as_deref() else {
            return Err(RuntimeError::new("import_failed"));
        };
        if !restore {
            import_runtime.decline_restore_selected_document(
                &outcome.document_id,
                intake_item_id,
            )?;
            return Ok(None);
        }
        import_runtime
            .confirm_restore_selected_document(&path, &outcome.document_id, intake_item_id)
            .map(Some)
        },
    )
    .await
    .map_err(|_| VaultCommandError::new("runtime_unavailable"))?
    .map_err(VaultCommandError::from)?;
    if outcome.is_some() {
        schedule_queued_local_inbox_parses(app, runtime);
    }
    Ok(outcome)
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
    run_runtime_task(move || -> Result<bool, RuntimeError> {
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
}

#[tauri::command]
pub(crate) async fn list_unassigned_source_documents(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<SourceDocumentSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_unassigned_source_documents()).await
}

#[tauri::command]
pub(crate) async fn list_source_documents(
    money_source_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<SourceDocumentSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_source_documents(&money_source_id)).await
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
    run_runtime_task(move || runtime.render_source_document_page(&document_id, page_number)).await
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
    run_runtime_task(move || runtime.preview_source_document(&document_id)).await
}
