use super::intake::validate_identifier;
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RestoreDecisionState {
    Capture,
    AlreadyRestored,
}

impl ManualImportStore {
    pub(crate) fn persist_confirmed_restore(
        &mut self,
        input: &SourceDocumentImport<'_>,
        stored: &StoredFile,
        document_id: &str,
        intake_item_id: &str,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        validate_identifier(document_id, "source document id")?;
        validate_identifier(intake_item_id, "intake item id")?;
        validate_import(input)?;
        let transaction = self.connection.transaction()?;
        let receipt_document_id: Option<String> = transaction
            .query_row(
                "SELECT source_document_id FROM intake_batch_items WHERE id = ?1",
                [intake_item_id],
                |row| row.get(0),
            )
            .optional()?;
        let receipt_outcome: Option<String> = transaction
            .query_row(
                "SELECT capture_outcome FROM intake_batch_items WHERE id = ?1",
                [intake_item_id],
                |row| row.get(0),
            )
            .optional()?;
        if receipt_document_id.as_deref() != Some(document_id)
            || receipt_outcome.as_deref() != Some("restore_confirmation_required")
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision receipt does not match the document",
            )
            .into());
        }
        let declined = transaction
            .query_row(
                "SELECT 1 FROM audit_log \
                 WHERE id = ?1 AND entity_type = 'source_document' \
                   AND entity_id = ?2 AND action = 'source_file_restore_declined' \
                 LIMIT 1",
                params![
                    restore_decision_audit_id(intake_item_id, "source_file_restore_declined"),
                    document_id
                ],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if declined {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision was already declined",
            )
            .into());
        }
        let Some(existing) = find_document_by_id(&transaction, document_id)? else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "restore decision document is missing",
            )
            .into());
        };
        if existing.file_sha256 != stored.file_sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision file does not match the receipt",
            )
            .into());
        }
        let restored = transaction
            .query_row(
                "SELECT 1 FROM audit_log \
                 WHERE id = ?1 AND entity_type = 'source_document' \
                   AND entity_id = ?2 AND action = 'source_file_restored' \
                 LIMIT 1",
                params![
                    restore_decision_audit_id(intake_item_id, "source_file_restored"),
                    document_id
                ],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if restored {
            if existing.file_state == "available" {
                transaction.rollback()?;
                return Ok(SourceDocumentImportOutcome {
                    document_id: document_id.to_owned(),
                    status: SourceDocumentImportStatus::Restored,
                    intake_item_id: Some(intake_item_id.to_owned()),
                });
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision document is not available",
            )
            .into());
        }
        if existing.file_state == "available" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision document is already available",
            )
            .into());
        }
        if existing.file_state != "deleted" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision document is not deleted",
            )
            .into());
        }
        let changed = transaction.execute(
            "UPDATE source_documents \
             SET encrypted_locator = ?1, file_state = 'available', \
                 deleted_at = NULL, deletion_audit_id = NULL \
             WHERE id = ?2 AND file_state = 'deleted'",
            params![stored.encrypted_locator, document_id],
        )?;
        if changed != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision document changed concurrently",
            )
            .into());
        }
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) VALUES (?1, 'source_document', ?2, 'source_file_restored', \
                       'user', 'user_confirmed_restore', ?3, 'source-file-restore-v1')",
            params![
                restore_decision_audit_id(intake_item_id, "source_file_restored"),
                document_id,
                stored.file_sha256
            ],
        )?;
        enqueue_parse_document(&transaction, document_id, &new_database_id("parse-run"))?;
        transaction.commit()?;
        Ok(SourceDocumentImportOutcome {
            document_id: document_id.to_owned(),
            status: SourceDocumentImportStatus::Restored,
            intake_item_id: Some(intake_item_id.to_owned()),
        })
    }

    pub(crate) fn record_restore_declined(
        &mut self,
        document_id: &str,
        intake_item_id: &str,
    ) -> StoreResult<()> {
        validate_identifier(document_id, "source document id")?;
        validate_identifier(intake_item_id, "intake item id")?;
        let transaction = self.connection.transaction()?;
        let receipt_document_id: Option<String> = transaction
            .query_row(
                "SELECT source_document_id FROM intake_batch_items WHERE id = ?1",
                [intake_item_id],
                |row| row.get(0),
            )
            .optional()?;
        let receipt_outcome: Option<String> = transaction
            .query_row(
                "SELECT capture_outcome FROM intake_batch_items WHERE id = ?1",
                [intake_item_id],
                |row| row.get(0),
            )
            .optional()?;
        if receipt_document_id.as_deref() != Some(document_id)
            || receipt_outcome.as_deref() != Some("restore_confirmation_required")
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision receipt does not match the document",
            )
            .into());
        }
        let restored = transaction
            .query_row(
                "SELECT 1 FROM audit_log \
                 WHERE id = ?1 AND entity_type = 'source_document' \
                   AND entity_id = ?2 AND action = 'source_file_restored' \
                 LIMIT 1",
                params![
                    restore_decision_audit_id(intake_item_id, "source_file_restored"),
                    document_id
                ],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if restored {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision was already confirmed",
            )
            .into());
        }
        let declined = transaction
            .query_row(
                "SELECT 1 FROM audit_log \
                 WHERE id = ?1 AND entity_type = 'source_document' \
                   AND entity_id = ?2 AND action = 'source_file_restore_declined' \
                 LIMIT 1",
                params![
                    restore_decision_audit_id(intake_item_id, "source_file_restore_declined"),
                    document_id
                ],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if declined {
            transaction.rollback()?;
            return Ok(());
        }
        let file_sha256: String = transaction
            .query_row(
                "SELECT file_sha256 FROM source_documents \
                 WHERE id = ?1 AND file_state = 'deleted'",
                [document_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "restore decision document is not deleted",
                )
            })?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) SELECT ?1, 'source_document', ?2, 'source_file_restore_declined', \
                       'user', 'user_declined_restore', ?3, 'source-file-restore-v1' \
             WHERE NOT EXISTS (SELECT 1 FROM audit_log WHERE id = ?1)",
            params![
                restore_decision_audit_id(intake_item_id, "source_file_restore_declined"),
                document_id,
                file_sha256
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn restore_decision_state(
        &self,
        document_id: &str,
        intake_item_id: &str,
        file_sha256: &str,
    ) -> StoreResult<RestoreDecisionState> {
        validate_identifier(document_id, "source document id")?;
        validate_identifier(intake_item_id, "intake item id")?;
        let receipt: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT source_document_id, capture_outcome \
                 FROM intake_batch_items WHERE id = ?1",
                [intake_item_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if receipt
            .as_ref()
            .map(|(id, outcome)| (id.as_str(), outcome.as_str()))
            != Some((document_id, "restore_confirmation_required"))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision receipt does not match the document",
            )
            .into());
        }
        let declined = self
            .connection
            .query_row(
                "SELECT 1 FROM audit_log \
                 WHERE id = ?1 AND entity_type = 'source_document' \
                   AND entity_id = ?2 AND action = 'source_file_restore_declined' \
                 LIMIT 1",
                params![
                    restore_decision_audit_id(intake_item_id, "source_file_restore_declined"),
                    document_id
                ],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if declined {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision was already declined",
            )
            .into());
        }
        let Some(existing) = find_document_by_id(&self.connection, document_id)? else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "restore decision document is missing",
            )
            .into());
        };
        if existing.file_sha256 != file_sha256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision file does not match the receipt",
            )
            .into());
        }
        let restored = self
            .connection
            .query_row(
                "SELECT 1 FROM audit_log \
                 WHERE id = ?1 AND entity_type = 'source_document' \
                   AND entity_id = ?2 AND action = 'source_file_restored' \
                 LIMIT 1",
                params![
                    restore_decision_audit_id(intake_item_id, "source_file_restored"),
                    document_id
                ],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if restored {
            if existing.file_state == "available" {
                return Ok(RestoreDecisionState::AlreadyRestored);
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision document is not available",
            )
            .into());
        }
        if existing.file_state != "deleted" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore decision document is not deleted",
            )
            .into());
        }
        Ok(RestoreDecisionState::Capture)
    }
}

fn restore_decision_audit_id(receipt_id: &str, action: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"cancan:source-restore-decision:v1\0");
    digest.update(receipt_id.as_bytes());
    digest.update([0]);
    digest.update(action.as_bytes());
    format!(
        "audit-restore-{}",
        digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
