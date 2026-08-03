use super::*;
use sha2::{Digest, Sha256};

struct StableLocalInboxCandidate {
    path: PathBuf,
    snapshot: FileSnapshot,
    entry_key: String,
    snapshot_version: String,
    intake_item_id: String,
    safe_input_label: String,
}

impl VaultRuntime {
    pub(crate) fn rescan_local_inbox(&self) -> Result<LocalInboxScanSummary, RuntimeError> {
        self.require_unlocked()?;
        #[cfg(test)]
        self.wait_for_local_inbox_scan_test_barrier();
        let _scan_guard = self
            .inner
            .local_inbox_scan_guard
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))?;
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
        let inbox = {
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
            inbox
        };
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
        let mut observed_candidates = Vec::new();
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
                Ok(snapshot) if !self.local_inbox_entry_is_current(&path, &snapshot)? => {
                    observed_candidates.push((path, snapshot));
                }
                Ok(_) => {}
                Err(_) => summary.deferred += 1,
            }
        }
        if !observed_candidates.is_empty() {
            std::thread::sleep(SETTLE_INTERVAL);
        }
        let mut candidates = Vec::new();
        for (path, first_snapshot) in observed_candidates {
            if !preflight.permits_read(&path) {
                summary.deferred += 1;
                continue;
            }
            let second_snapshot = match first_snapshot_after_preflight(&path, &preflight) {
                Ok(snapshot) => snapshot,
                Err(_) => {
                    summary.deferred += 1;
                    continue;
                }
            };
            if second_snapshot != first_snapshot {
                summary.deferred += 1;
                continue;
            }
            let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
                summary.deferred += 1;
                continue;
            };
            candidates.push(StableLocalInboxCandidate {
                entry_key: local_inbox_entry_key(&path)?,
                snapshot_version: local_inbox_snapshot_version(&first_snapshot),
                intake_item_id: random_identifier("intake-item"),
                safe_input_label: bounded_safe_input_label(filename),
                path,
                snapshot: first_snapshot,
            });
        }
        candidates.sort_by(|left, right| {
            left.entry_key
                .cmp(&right.entry_key)
                .then_with(|| left.safe_input_label.cmp(&right.safe_input_label))
        });
        if candidates.is_empty() {
            *self
                .inner
                .local_inbox_last_scan
                .lock()
                .map_err(|_| RuntimeError::new("local_inbox_unavailable"))? = Some(summary.clone());
            self.inner
                .local_inbox_needs_attention
                .store(false, Ordering::SeqCst);
            return Ok(summary);
        }
        let batch_id = random_identifier("intake-batch");
        let items = candidates
            .iter()
            .map(|candidate| IntakeBatchItemInput {
                id: &candidate.intake_item_id,
                safe_input_label: &candidate.safe_input_label,
                acquisition_input_key: Some(&candidate.entry_key),
                acquisition_input_version: Some(&candidate.snapshot_version),
                retry_of_batch_item_id: None,
            })
            .collect::<Vec<_>>();
        {
            let mut store = self.store()?;
            store
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?
                .create_intake_batch(&IntakeBatchInput {
                    id: &batch_id,
                    acquisition_channel: IntakeAcquisitionChannel::LocalInbox,
                    items: &items,
                })
                .map_err(|_| RuntimeError::new("local_inbox_scan_failed"))?;
        }
        let mut first_error = None;
        for candidate in candidates {
            let mut persisted = None;
            let outcome = capture_after_second_scan(
                &candidate.path,
                candidate.snapshot.clone(),
                &tombstoned_hashes,
                |bytes| match self.register_local_inbox_capture(
                    &candidate.path,
                    bytes,
                    &candidate.intake_item_id,
                ) {
                    Ok(outcome) => {
                        persisted = Some(outcome);
                        Ok(())
                    }
                    Err(_) => Err(io::Error::other("local Inbox import failed")),
                },
            );
            match &outcome {
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
                    None => {
                        if let Err(error) = self.finalize_intake_rejection_with_kind(
                            &candidate.intake_item_id,
                            "capture_failed",
                            IntakeRejectionKind::BackgroundActionRequired,
                        ) {
                            first_error.get_or_insert(error);
                        }
                        summary.deferred += 1;
                    }
                },
                CaptureOutcome::Suppressed { .. } => {
                    if let Err(error) = self.finalize_local_inbox_suppressed(
                        &candidate.intake_item_id,
                        "tombstone_suppressed",
                    ) {
                        first_error.get_or_insert(error);
                    }
                    summary.suppressed += 1;
                }
                CaptureOutcome::Deferred(reason) => {
                    if let Err(error) = self.finalize_intake_rejection_with_kind(
                        &candidate.intake_item_id,
                        local_inbox_defer_code(*reason),
                        IntakeRejectionKind::BackgroundActionRequired,
                    ) {
                        first_error.get_or_insert(error);
                    }
                    summary.deferred += 1;
                }
            }
            if matches!(
                outcome,
                CaptureOutcome::Captured { .. } | CaptureOutcome::Suppressed { .. }
            ) && let Err(error) =
                self.record_local_inbox_entry_observation(&candidate.path, &candidate.snapshot)
            {
                first_error.get_or_insert(error);
            }
        }
        if first_error.is_some() {
            let recovery = {
                let mut store = self.store()?;
                store
                    .as_mut()
                    .ok_or_else(|| RuntimeError::new("vault_locked"))?
                    .recover_pending_local_inbox_items(&batch_id)
                    .map_err(|_| RuntimeError::new("intake_recovery_failed"))
            };
            if let Err(error) = recovery {
                first_error.get_or_insert(error);
            }
        }
        *self
            .inner
            .local_inbox_last_scan
            .lock()
            .map_err(|_| RuntimeError::new("local_inbox_unavailable"))? = Some(summary.clone());
        self.inner
            .local_inbox_needs_attention
            .store(false, Ordering::SeqCst);
        match first_error {
            Some(error) => Err(error),
            None => Ok(summary),
        }
    }

    fn local_inbox_entry_is_current(
        &self,
        path: &Path,
        snapshot: &FileSnapshot,
    ) -> Result<bool, RuntimeError> {
        let entry_key = local_inbox_entry_key(path)?;
        let store = self.store()?;
        store
            .as_ref()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .local_inbox_entry_is_current(&entry_key, snapshot)
            .map_err(|_| RuntimeError::new("local_inbox_scan_failed"))
    }

    fn record_local_inbox_entry_observation(
        &self,
        path: &Path,
        snapshot: &FileSnapshot,
    ) -> Result<(), RuntimeError> {
        let entry_key = local_inbox_entry_key(path)?;
        #[cfg(test)]
        if self.take_observation_test_fault(path) {
            return Err(RuntimeError::new("local_inbox_observation_failed"));
        }
        let mut store = self.store()?;
        store
            .as_mut()
            .ok_or_else(|| RuntimeError::new("vault_locked"))?
            .record_local_inbox_entry_observation(&entry_key, snapshot)
            .map_err(|_| RuntimeError::new("local_inbox_scan_failed"))
    }

    fn finalize_local_inbox_suppressed(
        &self,
        item_id: &str,
        code: &'static str,
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
            .finalize_intake_batch_item(item_id, IntakeItemFinalization::Suppressed { code })
            .map_err(|_| RuntimeError::new("intake_finalization_failed"))
    }

    pub(super) fn register_local_inbox_capture(
        &self,
        source_path: &Path,
        captured_bytes: Zeroizing<Vec<u8>>,
        intake_item_id: &str,
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
            #[cfg(test)]
            source_path,
        };
        let session_generation = self.inner.vault_session_generation.load(Ordering::SeqCst);
        let source = ManualImportStore::prepare_source_bytes(mime_type, captured_bytes)
            .map_err(|_| RuntimeError::new("local_inbox_import_failed"))?;
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != session_generation {
            return Err(RuntimeError::new("vault_locked"));
        }
        let plan = {
            let mut store = self.store()?;
            store
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?
                .intake_source_capture_plan(&input, &source, intake_item_id, true)
                .map_err(|_| RuntimeError::new("local_inbox_import_failed"))?
        };
        let capture = match plan {
            SourceCapturePlan::Capture(capture) => capture,
            SourceCapturePlan::RestoreConfirmationRequired(outcome) => return Ok(outcome),
        };
        let stored = capture
            .store_prepared(&source)
            .map_err(|_| RuntimeError::new("local_inbox_import_failed"))?;
        if self.inner.vault_session_generation.load(Ordering::SeqCst) != session_generation {
            return Err(RuntimeError::new("vault_locked"));
        }
        let result = {
            let mut store = self.store()?;
            store
                .as_mut()
                .ok_or_else(|| RuntimeError::new("vault_locked"))?
                .persist_captured_intake_import(&input, &stored, intake_item_id)
        };
        result.map_err(|_| RuntimeError::new("local_inbox_import_failed"))
    }
}

fn local_inbox_entry_key(path: &Path) -> Result<String, RuntimeError> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| RuntimeError::new("local_inbox_scan_failed"))?;
    Ok(format!("{:x}", Sha256::digest(name.as_bytes())))
}

pub(super) fn local_inbox_snapshot_version(snapshot: &FileSnapshot) -> String {
    let mut digest = Sha256::new();
    digest.update(b"cancan:local-inbox-snapshot:v1\0");
    update_snapshot_field(
        &mut digest,
        b"creation_seconds:i64",
        &snapshot.creation_seconds.to_be_bytes(),
    );
    update_snapshot_field(
        &mut digest,
        b"creation_nanoseconds:i64",
        &snapshot.creation_nanoseconds.to_be_bytes(),
    );
    update_snapshot_field(
        &mut digest,
        b"change_seconds:i64",
        &snapshot.change_seconds.to_be_bytes(),
    );
    update_snapshot_field(
        &mut digest,
        b"change_nanoseconds:i64",
        &snapshot.change_nanoseconds.to_be_bytes(),
    );
    update_snapshot_field(
        &mut digest,
        b"modified_seconds:i64",
        &snapshot.modified_seconds.to_be_bytes(),
    );
    update_snapshot_field(
        &mut digest,
        b"modified_nanoseconds:i64",
        &snapshot.modified_nanoseconds.to_be_bytes(),
    );
    update_snapshot_field(&mut digest, b"size:u64", &snapshot.size.to_be_bytes());
    update_snapshot_field(
        &mut digest,
        b"device:u64",
        &snapshot.identity.device.to_be_bytes(),
    );
    update_snapshot_field(
        &mut digest,
        b"inode:u64",
        &snapshot.identity.inode.to_be_bytes(),
    );
    let hex = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("v1:{hex}")
}

fn update_snapshot_field(digest: &mut Sha256, tag: &[u8], value: &[u8]) {
    digest.update([u8::try_from(tag.len()).expect("snapshot field tag fits in one byte")]);
    digest.update(tag);
    digest.update(value);
}

fn local_inbox_defer_code(reason: CaptureDeferReason) -> &'static str {
    match reason {
        CaptureDeferReason::ChangedAfterRead | CaptureDeferReason::ChangedBeforeRead => {
            "capture_deferred"
        }
        CaptureDeferReason::NativePreflight => "native_preflight_deferred",
        CaptureDeferReason::Unreadable => "capture_failed",
        CaptureDeferReason::Unsupported => "unsupported_input",
    }
}
