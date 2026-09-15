use super::*;

impl ManualImportStore {
    /// Closes review work attached to a committed record. The committed record
    /// is terminal and stays untouched: this writes only the review item status
    /// and its audit entry, and it refuses every other record state so an
    /// uncommitted record keeps its edit/remove paths.
    pub(crate) fn acknowledge_review_item(
        &mut self,
        review_item_id: &str,
        expected_record_version: i64,
    ) -> StoreResult<ReviewMutationOutcome> {
        if expected_record_version <= 0 {
            return Ok(review_conflict("invalid_review_request"));
        }
        let transaction = self.connection.transaction()?;
        let current = transaction
            .query_row(
                "SELECT external_records.id FROM review_items \
                 JOIN external_records ON external_records.id = review_items.external_record_id \
                 WHERE review_items.id = ?1 AND review_items.status = 'open' \
                   AND external_records.version = ?2 \
                   AND external_records.status = 'committed'",
                params![review_item_id, expected_record_version],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(record_id) = current else {
            return Ok(review_conflict("stale_review_item"));
        };
        transaction.execute(
            "UPDATE review_items SET status = 'dismissed' WHERE id = ?1",
            [review_item_id],
        )?;
        transaction.execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
             ) VALUES (?1, 'external_record', ?2, 'review_item_acknowledged', 'user', \
                       'review_acknowledge', ?3, ?4)",
            params![
                new_audit_id(),
                record_id,
                format!("{review_item_id}:{expected_record_version}"),
                REVIEW_POLICY_VERSION,
            ],
        )?;
        transaction.commit()?;
        Ok(ReviewMutationOutcome {
            reason: None,
            record_version: None,
            review_item_id: None,
            status: ReviewMutationStatus::Acknowledged,
        })
    }
}
