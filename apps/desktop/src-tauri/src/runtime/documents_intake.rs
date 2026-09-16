use super::*;

pub(super) fn source_document_metadata(
    runtime: &VaultRuntime,
    source_path: &Path,
) -> Result<(String, &'static str), RuntimeError> {
    let metadata = fs::metadata(source_path).map_err(|error| {
        runtime_failure(runtime, "source_document_metadata", "import_failed", &error)
    })?;
    if !metadata.is_file() {
        return Err(RuntimeError::new("unsupported_document"));
    }
    if metadata.len() > crate::source_file::MAX_SOURCE_FILE_BYTES {
        return Err(RuntimeError::new("source_file_too_large"));
    }
    source_document_filename_metadata(source_path)
}

pub(super) fn source_document_filename_metadata(
    source_path: &Path,
) -> Result<(String, &'static str), RuntimeError> {
    let mime_type = match source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("pdf") => "application/pdf",
        Some("csv") => "text/csv",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => return Err(RuntimeError::new("unsupported_document")),
    };
    let original_filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| RuntimeError::new("unsupported_document"))?
        .to_owned();
    Ok((original_filename, mime_type))
}

impl VaultRuntime {
    pub(super) fn finalize_intake_rejection(
        &self,
        item_id: &str,
        code: &'static str,
    ) -> Result<(), RuntimeError> {
        self.finalize_intake_rejection_with_kind(item_id, code, IntakeRejectionKind::VisibleReceipt)
    }

    pub(super) fn finalize_intake_rejection_with_kind(
        &self,
        item_id: &str,
        code: &'static str,
        kind: IntakeRejectionKind,
    ) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        #[cfg(test)]
        if self.take_intake_test_fault(IntakeTestFault::Finalization) {
            return Err(RuntimeError::new("intake_finalization_failed"));
        }
        store
            .finalize_intake_batch_item(
                item_id,
                IntakeItemFinalization::Rejected {
                    code,
                    kind,
                    parked: false,
                },
            )
            .map_store_error(
                store,
                "finalize_intake_batch_item",
                "intake_finalization_failed",
            )
    }

    pub(crate) fn import_selected_document(
        &self,
        source_path: &Path,
    ) -> Result<SourceDocumentImportOutcome, RuntimeError> {
        let document_id = random_identifier("document");
        let audit_id = random_identifier("audit");
        let batch_id = random_identifier("intake-batch");
        let intake_item_id = random_identifier("intake-item");
        let safe_input_label = explicit_safe_input_label(source_path);
        {
            let mut store = self.store()?;
            let store = store
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            store
                .create_intake_batch(&IntakeBatchInput {
                    id: &batch_id,
                    acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
                    items: &[IntakeBatchItemInput {
                        id: &intake_item_id,
                        safe_input_label: &safe_input_label,
                        acquisition_input_key: None,
                        acquisition_input_version: None,
                        retry_of_batch_item_id: None,
                    }],
                })
                .map_store_error(store, "create_intake_batch", "import_failed")?;
        }
        let (original_filename, mime_type) = match source_document_metadata(self, source_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                let code = error.code;
                return Err(self
                    .finalize_intake_rejection(&intake_item_id, code)
                    .err()
                    .unwrap_or(error));
            }
        };
        let input = SourceDocumentImport {
            audit_actor: "user",
            audit_id: &audit_id,
            audit_policy_version: IMPORT_POLICY_VERSION,
            audit_reason: "manual_import",
            document_id: &document_id,
            mime_type,
            original_filename: &original_filename,
            #[cfg(test)]
            source_path,
        };
        let session_generation = self.inner.vault_session_generation.load(Ordering::SeqCst);
        let source = match ManualImportStore::prepare_source_path(mime_type, source_path) {
            Ok(source) => source,
            Err(_) => {
                let finalization = self.finalize_intake_rejection(&intake_item_id, "import_failed");
                return Err(finalization
                    .err()
                    .unwrap_or_else(|| RuntimeError::new("import_failed")));
            }
        };
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != session_generation {
            let finalization = self.finalize_intake_rejection_with_kind(
                &intake_item_id,
                "import_interrupted",
                IntakeRejectionKind::BackgroundActionRequired,
            );
            return Err(finalization
                .err()
                .unwrap_or_else(|| RuntimeError::new("vault_locked")));
        }
        let plan_result = (|| -> Result<SourceCapturePlan, RuntimeError> {
            let mut store = self.store()?;
            let store = store
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            #[cfg(test)]
            if self.take_intake_test_fault(IntakeTestFault::Plan) {
                Err(RuntimeError::new("import_failed"))
            } else {
                store
                    .intake_source_capture_plan(&input, &source, &intake_item_id, false)
                    .map_store_error(store, "intake_source_capture_plan", "import_failed")
            }
            #[cfg(not(test))]
            store
                .intake_source_capture_plan(&input, &source, &intake_item_id, false)
                .map_store_error(store, "intake_source_capture_plan", "import_failed")
        })();
        let plan = match plan_result {
            Ok(plan) => plan,
            Err(_) => {
                let finalization = self.finalize_intake_rejection(&intake_item_id, "import_failed");
                return Err(finalization
                    .err()
                    .unwrap_or_else(|| RuntimeError::new("import_failed")));
            }
        };
        let capture = match plan {
            SourceCapturePlan::Capture(capture) => capture,
            SourceCapturePlan::RestoreConfirmationRequired(outcome) => return Ok(outcome),
        };
        let stored = match capture.store_prepared(&source) {
            Ok(stored) => stored,
            Err(_) => {
                let finalization = self.finalize_intake_rejection(&intake_item_id, "import_failed");
                return Err(finalization
                    .err()
                    .unwrap_or_else(|| RuntimeError::new("import_failed")));
            }
        };
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != session_generation {
            let finalization = self.finalize_intake_rejection_with_kind(
                &intake_item_id,
                "import_interrupted",
                IntakeRejectionKind::BackgroundActionRequired,
            );
            return Err(finalization
                .err()
                .unwrap_or_else(|| RuntimeError::new("vault_locked")));
        }
        let persist_result = (|| -> Result<SourceDocumentImportOutcome, RuntimeError> {
            let mut store = self.store()?;
            let store = store
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            #[cfg(test)]
            if self.take_intake_test_fault(IntakeTestFault::Persist) {
                Err(RuntimeError::new("import_failed"))
            } else {
                store
                    .persist_captured_intake_import(&input, &stored, &intake_item_id)
                    .map_store_error(store, "persist_captured_intake_import", "import_failed")
            }
            #[cfg(not(test))]
            store
                .persist_captured_intake_import(&input, &stored, &intake_item_id)
                .map_store_error(store, "persist_captured_intake_import", "import_failed")
        })();
        match persist_result {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                let finalization =
                    self.finalize_intake_rejection(&intake_item_id, error_code(&error));
                Err(finalization.err().unwrap_or(error))
            }
        }
    }

    pub(crate) fn confirm_restore_selected_document(
        &self,
        source_path: &Path,
        document_id: &str,
        intake_item_id: &str,
    ) -> Result<SourceDocumentImportOutcome, RuntimeError> {
        let (original_filename, mime_type) = source_document_metadata(self, source_path)?;
        let audit_id = random_identifier("audit");
        let input = SourceDocumentImport {
            audit_actor: "user",
            audit_id: &audit_id,
            audit_policy_version: "source-file-restore-v1",
            audit_reason: "user_confirmed_restore",
            document_id,
            mime_type,
            original_filename: &original_filename,
            #[cfg(test)]
            source_path,
        };
        let session_generation = self.inner.vault_session_generation.load(Ordering::SeqCst);
        let source =
            ManualImportStore::prepare_source_path(mime_type, source_path).map_err(|error| {
                runtime_failure(self, "prepare_source_path", "import_failed", &*error)
            })?;
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != session_generation {
            return Err(RuntimeError::new("vault_locked"));
        }
        let capture = {
            let store = self.store()?;
            let store = store
                .as_ref()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?;
            match store
                .restore_decision_state(document_id, intake_item_id, source.file_sha256())
                .map_store_error(store, "restore_decision_state", "import_failed")?
            {
                RestoreDecisionState::AlreadyRestored => {
                    return Ok(SourceDocumentImportOutcome {
                        document_id: document_id.to_owned(),
                        status: SourceDocumentImportStatus::Restored,
                        intake_item_id: Some(intake_item_id.to_owned()),
                    });
                }
                RestoreDecisionState::Capture => {}
            }
            if self.inner.vault_session_generation.load(Ordering::SeqCst) != session_generation {
                return Err(RuntimeError::new("vault_locked"));
            }
            match store
                .source_capture_plan(&input, &source, Some(document_id))
                .map_store_error(store, "source_capture_plan", "import_failed")?
            {
                SourceCapturePlan::Capture(capture) => capture,
                SourceCapturePlan::RestoreConfirmationRequired(_) => {
                    return Err(RuntimeError::new("import_failed"));
                }
            }
        };
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != session_generation {
            return Err(RuntimeError::new("vault_locked"));
        }
        let stored = capture
            .store_prepared(&source)
            .map_err(|error| runtime_failure(self, "store_prepared", "import_failed", &*error))?;
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != session_generation {
            return Err(RuntimeError::new("vault_locked"));
        }
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .persist_confirmed_restore(&input, &stored, document_id, intake_item_id)
            .map_store_error(store, "persist_confirmed_restore", "import_failed")
    }

    pub(crate) fn decline_restore_selected_document(
        &self,
        document_id: &str,
        intake_item_id: &str,
    ) -> Result<(), RuntimeError> {
        let mut store = self.store()?;
        let store = store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?;
        store
            .record_restore_declined(document_id, intake_item_id)
            .map_store_error(store, "record_restore_declined", "import_failed")
    }
}

fn explicit_safe_input_label(source_path: &Path) -> String {
    source_path
        .file_name()
        .map(|name| bounded_safe_input_label(&name.to_string_lossy()))
        .unwrap_or_else(|| bounded_safe_input_label("File"))
}

fn error_code(error: &RuntimeError) -> &'static str {
    match error.code {
        "vault_locked" | "runtime_unavailable" | "intake_finalization_failed" => "import_failed",
        code => code,
    }
}
