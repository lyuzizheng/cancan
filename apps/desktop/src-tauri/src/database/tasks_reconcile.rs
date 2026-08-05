use super::tasks::DownstreamState;
use super::*;

impl ManualImportStore {
    pub(crate) fn reconcile_sealed_batches(&mut self) -> StoreResult<()> {
        let batch_ids: Vec<String> = self
            .connection
            .prepare("SELECT id FROM intake_batches WHERE completed_at IS NULL")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let declined_restore_item_ids = self.restore_declined_item_ids()?;

        for batch_id in batch_ids {
            let transaction = self.connection.transaction()?;
            let already_completed: Option<i64> = transaction
                .query_row(
                    "SELECT 1 FROM intake_batches WHERE id = ?1 AND completed_at IS NOT NULL",
                    [&batch_id],
                    |_| Ok(1),
                )
                .optional()?;
            if already_completed.is_some() {
                continue;
            }

            let mut statement = transaction.prepare(
                "SELECT i.capture_outcome, i.source_document_id, i.rejection_kind, \
                        i.rejection_parked_at, b.acquisition_channel, i.id \
                 FROM intake_batch_items i \
                 JOIN intake_batches b ON b.id = i.intake_batch_id \
                 WHERE i.intake_batch_id = ?1",
            )?;
            struct Item {
                capture_outcome: String,
                source_document_id: Option<String>,
                rejection_kind: Option<String>,
                rejection_parked_at: Option<String>,
                acquisition_channel: String,
                item_id: String,
            }
            let items = statement
                .query_map([&batch_id], |row| {
                    Ok(Item {
                        capture_outcome: row.get(0)?,
                        source_document_id: row.get(1)?,
                        rejection_kind: row.get(2)?,
                        rejection_parked_at: row.get(3)?,
                        acquisition_channel: row.get(4)?,
                        item_id: row.get(5)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            drop(statement);

            let mut all_terminal = true;
            let mut any_user_meaningful = false;

            for item in items {
                if item.capture_outcome == "pending" {
                    all_terminal = false;
                    break;
                }

                if item.capture_outcome == "captured" {
                    if let Some(document_id) = &item.source_document_id {
                        if Self::document_has_active_pipeline(&transaction, document_id)? {
                            all_terminal = false;
                            break;
                        }
                        let state =
                            Self::captured_document_downstream_state(&transaction, document_id)?;
                        if matches!(state, DownstreamState::Ready | DownstreamState::Actionable) {
                            any_user_meaningful = true;
                        }
                    }
                } else if item.capture_outcome == "restore_confirmation_required" {
                    // A declined restore decision is recorded per intake item and is
                    // not user-meaningful for notification purposes; the task list no
                    // longer shows a row for it.
                    if !declined_restore_item_ids.contains(&item.item_id) {
                        any_user_meaningful = true;
                    }
                } else if item.capture_outcome == "rejected"
                    && item.rejection_kind.as_deref() == Some("background_action_required")
                {
                    // Parked local-inbox rejections only surface in the Parked group,
                    // so they must not hold a batch in `pending` notification state.
                    let parked_local_inbox = item.acquisition_channel == "local_inbox"
                        && item.rejection_parked_at.is_some();
                    if !parked_local_inbox {
                        any_user_meaningful = true;
                    }
                }
            }

            if all_terminal {
                let notification_state = if any_user_meaningful {
                    "pending"
                } else {
                    "suppressed"
                };
                transaction.execute(
                    "UPDATE intake_batches \
                     SET completed_at = CURRENT_TIMESTAMP, \
                         notification_state = ?1, \
                         notification_decided_at = CURRENT_TIMESTAMP \
                     WHERE id = ?2 AND completed_at IS NULL",
                    params![notification_state, batch_id],
                )?;
                transaction.commit()?;
            }
        }

        Ok(())
    }

    fn document_has_active_pipeline(
        transaction: &Transaction,
        document_id: &str,
    ) -> StoreResult<bool> {
        let active: bool = transaction.query_row(
            "SELECT EXISTS( \
               SELECT 1 FROM jobs \
               WHERE related_source_document_id = ?1 \
                 AND job_type IN ('parse_document', 'reconcile_document') \
                 AND status IN ('queued', 'running') \
             )",
            [document_id],
            |row| row.get(0),
        )?;
        Ok(active)
    }

    /// True only when a captured document has fully cleared the pipeline and is
    /// genuinely Ready: file available, money source assigned (not a candidate or
    /// parked), a successful reconcile on record, no non-succeeded pipeline job, and
    /// no open review items. Mirrors the predicate used by `derive_ready`.
    fn document_is_ready(transaction: &Transaction, document_id: &str) -> StoreResult<bool> {
        let ready: bool = transaction.query_row(
            "SELECT EXISTS( \
               SELECT 1 FROM source_documents sd \
               WHERE sd.id = ?1 \
                 AND sd.file_state = 'available' \
                 AND sd.money_source_id IS NOT NULL \
                 AND sd.money_source_candidate_id IS NULL \
                 AND sd.attention_parked_reason IS NULL \
                 AND EXISTS ( \
                   SELECT 1 FROM jobs \
                   WHERE related_source_document_id = sd.id \
                     AND job_type = 'reconcile_document' \
                     AND status = 'succeeded' \
                 ) \
                 AND NOT EXISTS ( \
                   SELECT 1 FROM jobs \
                   WHERE related_source_document_id = sd.id \
                     AND job_type IN ('parse_document', 'reconcile_document') \
                     AND status != 'succeeded' \
                 ) \
                 AND NOT EXISTS ( \
                   SELECT 1 FROM review_items ri \
                   JOIN external_records er ON er.id = ri.external_record_id \
                   WHERE er.source_document_id = sd.id \
                     AND ri.status = 'open' \
                     AND er.status IN ('staged', 'review') \
                 ) \
             )",
            [document_id],
            |row| row.get(0),
        )?;
        Ok(ready)
    }

    fn captured_document_downstream_state(
        transaction: &Transaction,
        document_id: &str,
    ) -> StoreResult<DownstreamState> {
        let row: (Option<String>, Option<String>, Option<String>, Option<String>) =
            transaction.query_row(
                "SELECT money_source_id, money_source_candidate_id, attention_parked_reason, \
                        (SELECT status FROM money_source_candidates WHERE id = source_documents.money_source_candidate_id) AS candidate_status \
                 FROM source_documents \
                 WHERE id = ?1",
                [document_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                    ))
                },
            )?;

        let (money_source_id, candidate_id, attention_parked, candidate_status) = row;

        if attention_parked.as_deref() == Some("statement_password") {
            return Ok(DownstreamState::Other);
        }

        if candidate_id.is_some() {
            if candidate_status.as_deref() == Some("pending") {
                return Ok(DownstreamState::Actionable);
            }
            // kept_unassigned or unknown -> parked/other
            return Ok(DownstreamState::Other);
        }

        let blocked: Option<String> = transaction
            .query_row(
                "SELECT blocked_reason FROM jobs \
                 WHERE related_source_document_id = ?1 \
                   AND job_type = 'parse_document' \
                 ORDER BY created_at DESC, rowid DESC LIMIT 1",
                [document_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();

        if blocked.as_deref() == Some("password_required") {
            return Ok(DownstreamState::Actionable);
        }

        if blocked.is_some() {
            return Ok(DownstreamState::Actionable);
        }

        let has_open_review: bool = transaction.query_row(
            "SELECT EXISTS( \
               SELECT 1 FROM review_items ri \
               JOIN external_records er ON er.id = ri.external_record_id \
               WHERE er.source_document_id = ?1 \
                 AND ri.status = 'open' \
                 AND er.status IN ('staged', 'review') \
             )",
            [document_id],
            |row| row.get(0),
        )?;
        if has_open_review {
            return Ok(DownstreamState::Actionable);
        }

        if money_source_id.is_some() {
            // A money source alone does not make the document Ready: reconcile must
            // have succeeded. If it has not (e.g. failed reconcile), report Actionable
            // so the batch is actionable and the NeedsAttention row can surface.
            if Self::document_is_ready(transaction, document_id)? {
                return Ok(DownstreamState::Ready);
            }
            return Ok(DownstreamState::Actionable);
        }

        // Unassigned and no candidate/job -> needs attention
        Ok(DownstreamState::Actionable)
    }
}
