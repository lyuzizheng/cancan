#[cfg(test)]
use super::intake::validate_identifier;
use super::*;

impl ManualImportStore {
    #[cfg(test)]
    pub(crate) fn insert_local_inbox_rejected_batch_for_test(
        &mut self,
        batch_id: &str,
        item_id: &str,
        label: &str,
        finalized_at: &str,
        code: &str,
        parked: bool,
    ) -> StoreResult<()> {
        validate_identifier(batch_id, "intake batch id")?;
        validate_identifier(item_id, "intake item id")?;
        let (key, version) = (
            "0000000000000000000000000000000000000000000000000000000000000001",
            "v1:0000000000000000000000000000000000000000000000000000000000000001",
        );
        self.connection.execute(
            "INSERT INTO intake_batches(id, acquisition_channel, opened_at, sealed_at) \
             VALUES (?1, 'local_inbox', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            [batch_id],
        )?;
        self.connection.execute(
            "INSERT INTO intake_batch_items( \
               id, intake_batch_id, input_ordinal, safe_input_label, capture_outcome, \
               rejection_kind, rejection_code, acquisition_input_key, acquisition_input_version, \
               finalized_at, rejection_parked_at \
             ) VALUES (?1, ?2, 0, ?3, 'rejected', 'background_action_required', ?4, ?5, ?6, ?7, \
                       CASE WHEN ?8 THEN CURRENT_TIMESTAMP ELSE NULL END)",
            params![
                item_id,
                batch_id,
                label,
                code,
                key,
                version,
                finalized_at,
                parked
            ],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn backdate_reconcile_finished_at_for_test(
        &mut self,
        document_id: &str,
        finished_at: &str,
    ) -> StoreResult<()> {
        self.connection.execute(
            "UPDATE jobs SET finished_at = ?1 \
             WHERE related_source_document_id = ?2 \
               AND job_type = 'reconcile_document' AND status = 'succeeded'",
            params![finished_at, document_id],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn insert_visible_receipt_batch_for_test(
        &mut self,
        batch_id: &str,
        item_id: &str,
        label: &str,
        finalized_at: &str,
    ) -> StoreResult<()> {
        validate_identifier(batch_id, "intake batch id")?;
        validate_identifier(item_id, "intake item id")?;
        self.connection.execute(
            "INSERT INTO intake_batches(id, acquisition_channel, opened_at, sealed_at) \
             VALUES (?1, 'explicit_handoff', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            [batch_id],
        )?;
        self.connection.execute(
            "INSERT INTO intake_batch_items( \
               id, intake_batch_id, input_ordinal, safe_input_label, capture_outcome, \
               rejection_kind, rejection_code, finalized_at \
             ) VALUES (?1, ?2, 0, ?3, 'rejected', 'visible_receipt', 'unsupported', ?4)",
            params![item_id, batch_id, label, finalized_at],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn set_source_document_attention_parked_for_test(
        &mut self,
        document_id: &str,
        reason: &str,
        timestamp: &str,
    ) -> StoreResult<()> {
        validate_identifier(document_id, "source document id")?;
        self.connection.execute(
            "UPDATE source_documents \
             SET attention_parked_reason = ?1, attention_parked_at = ?2 \
             WHERE id = ?3",
            params![reason, timestamp, document_id],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn intake_batch_completion_for_test(
        &self,
    ) -> StoreResult<Vec<(String, Option<String>, String)>> {
        let mut statement = self.connection.prepare(
            "SELECT id, completed_at, notification_state \
             FROM intake_batches ORDER BY id",
        )?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
