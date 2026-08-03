use super::*;

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IntakeItemTestState {
    pub(crate) id: String,
    pub(crate) intake_batch_id: String,
    pub(crate) input_ordinal: i64,
    pub(crate) safe_input_label: String,
    pub(crate) capture_outcome: String,
    pub(crate) source_document_id: Option<String>,
    pub(crate) rejection_kind: Option<String>,
    pub(crate) rejection_code: Option<String>,
    pub(crate) rejection_resolved_at: Option<String>,
    pub(crate) resolved_by_batch_item_id: Option<String>,
    pub(crate) acquisition_input_key: Option<String>,
    pub(crate) acquisition_input_version: Option<String>,
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IntakeBatchTestState {
    pub(crate) id: String,
    pub(crate) acquisition_channel: String,
    pub(crate) item_count: i64,
    pub(crate) sealed_at: Option<String>,
}

impl ManualImportStore {
    #[cfg(test)]
    pub(crate) fn intake_item_test_states(&self) -> StoreResult<Vec<IntakeItemTestState>> {
        let mut statement = self.connection.prepare(
            "SELECT id, intake_batch_id, input_ordinal, safe_input_label, capture_outcome, source_document_id, \
                    rejection_kind, rejection_code, rejection_resolved_at, resolved_by_batch_item_id, \
                    acquisition_input_key, acquisition_input_version \
             FROM intake_batch_items ORDER BY intake_batch_id, input_ordinal",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(IntakeItemTestState {
                id: row.get(0)?,
                intake_batch_id: row.get(1)?,
                input_ordinal: row.get(2)?,
                safe_input_label: row.get(3)?,
                capture_outcome: row.get(4)?,
                source_document_id: row.get(5)?,
                rejection_kind: row.get(6)?,
                rejection_code: row.get(7)?,
                rejection_resolved_at: row.get(8)?,
                resolved_by_batch_item_id: row.get(9)?,
                acquisition_input_key: row.get(10)?,
                acquisition_input_version: row.get(11)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    #[cfg(test)]
    pub(crate) fn intake_batch_test_states(&self) -> StoreResult<Vec<IntakeBatchTestState>> {
        let mut statement = self.connection.prepare(
            "SELECT batches.id, batches.acquisition_channel, count(items.id), batches.sealed_at \
             FROM intake_batches AS batches \
             LEFT JOIN intake_batch_items AS items ON items.intake_batch_id = batches.id \
             GROUP BY batches.id ORDER BY batches.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(IntakeBatchTestState {
                id: row.get(0)?,
                acquisition_channel: row.get(1)?,
                item_count: row.get(2)?,
                sealed_at: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    #[cfg(test)]
    pub(crate) fn source_document_audit_actions_test(
        &self,
        document_id: &str,
    ) -> StoreResult<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT action FROM audit_log \
             WHERE entity_type = 'source_document' AND entity_id = ?1 \
             ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([document_id], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}
