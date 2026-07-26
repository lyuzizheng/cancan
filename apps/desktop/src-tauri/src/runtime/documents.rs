use super::*;

impl VaultRuntime {
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

    pub(super) fn save_statement_password_in_store(
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

    pub(super) fn save_verified_statement_password(
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

    pub(super) fn reconcile_statement_passwords(
        &self,
        store: &ManualImportStore,
    ) -> Result<(), RuntimeError> {
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
            .map_err(|_| RuntimeError::new("list_sources_failed"))
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

    pub(super) fn source_document_summary(
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

    pub(super) fn normalization_input(
        &self,
        document_id: &str,
    ) -> Result<ExtractionBundle, RuntimeError> {
        let store = self.store()?;
        let input = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .source_document_input(document_id)
            .map_err(|_| RuntimeError::new("document_unavailable"))?;
        let password = if input.mime_type == "application/pdf" {
            let passwords = self.document_passwords()?;
            passwords.get(document_id).cloned()
        } else {
            None
        };
        extract_bundle(
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
        // Keep the session mutex through rendering so Vault lock cannot report success
        // while this decrypted page buffer is still alive.
        let store = self.store()?;
        let input = store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .source_document_input(document_id)
            .map_err(|_| RuntimeError::new("document_unavailable"))?;
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

pub(super) fn ensure_statement_password_source(
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
pub(crate) async fn try_saved_statement_password(
    document_id: String,
    money_source_id: String,
    runtime: State<'_, VaultRuntime>,
) -> Result<SavedStatementPasswordResult, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.try_saved_statement_password(&document_id, &money_source_id))
        .await
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
    run_runtime_task(move || {
        runtime.unlock_source_document(
            &document_id,
            &money_source_id,
            password.as_bytes(),
            update_saved_password,
        )
    })
    .await
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
        let destination = selected
            .into_path()
            .map_err(|_| RuntimeError::new("file_selection_failed"))?;
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
    tauri::async_runtime::spawn_blocking(
        move || -> Result<Option<SourceDocumentImportOutcome>, RuntimeError> {
        runtime.require_unlocked()?;
        let selected = app
            .dialog()
            .file()
            .set_title("Import a statement")
            .add_filter("Financial documents", &["pdf", "csv", "png", "jpg", "jpeg"])
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
    let result = run_normalizer_sidecar(&app, &document_id, &input)
        .await
        .map_err(VaultCommandError::from)?;
    run_runtime_task(move || runtime.apply_normalizer_result(&document_id, &input, result)).await
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
pub(crate) async fn list_money_sources(
    runtime: State<'_, VaultRuntime>,
) -> Result<Vec<MoneySourceSummary>, VaultCommandError> {
    let runtime = runtime.inner().clone();
    run_runtime_task(move || runtime.list_money_sources()).await
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
