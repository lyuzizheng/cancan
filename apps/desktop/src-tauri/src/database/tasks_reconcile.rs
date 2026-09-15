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
                        i.rejection_parked_at, b.acquisition_channel, i.id, \
                        sd.id, sd.money_source_id, sd.money_source_candidate_id, \
                        sd.attention_parked_reason, \
                        EXISTS(SELECT 1 FROM jobs \
                               WHERE related_source_document_id = sd.id \
                                 AND job_type IN ('parse_document', 'reconcile_document') \
                                 AND status IN ('queued', 'running')), \
                        (SELECT status FROM money_source_candidates \
                          WHERE id = sd.money_source_candidate_id), \
                        (SELECT blocked_reason FROM jobs \
                          WHERE related_source_document_id = sd.id \
                            AND job_type = 'parse_document' \
                          ORDER BY created_at DESC, rowid DESC LIMIT 1), \
                        EXISTS(SELECT 1 FROM review_items ri \
                               JOIN external_records er ON er.id = ri.external_record_id \
                               WHERE er.source_document_id = sd.id \
                                 AND ri.status = 'open' \
                                 AND er.status IN ('staged', 'review')), \
                        EXISTS(SELECT 1 FROM source_documents ready \
                               WHERE ready.id = sd.id \
                                 AND ready.file_state = 'available' \
                                 AND ready.money_source_id IS NOT NULL \
                                 AND ready.money_source_candidate_id IS NULL \
                                 AND ready.attention_parked_reason IS NULL \
                                 AND EXISTS(SELECT 1 FROM jobs \
                                            WHERE related_source_document_id = ready.id \
                                              AND job_type = 'reconcile_document' \
                                              AND status = 'succeeded') \
                                 AND NOT EXISTS(SELECT 1 FROM jobs \
                                                WHERE related_source_document_id = ready.id \
                                                  AND job_type IN ('parse_document', 'reconcile_document') \
                                                  AND status != 'succeeded') \
                                 AND NOT EXISTS(SELECT 1 FROM review_items ri \
                                                JOIN external_records er ON er.id = ri.external_record_id \
                                                WHERE er.source_document_id = ready.id \
                                                  AND ri.status = 'open' \
                                                  AND er.status IN ('staged', 'review'))) \
                 FROM intake_batch_items i \
                 JOIN intake_batches b ON b.id = i.intake_batch_id \
                 LEFT JOIN source_documents sd ON sd.id = i.source_document_id \
                 WHERE i.intake_batch_id = ?1",
            )?;
            let items = statement
                .query_map([&batch_id], |row| {
                    Ok(BatchItem {
                        capture_outcome: row.get(0)?,
                        source_document_id: row.get(1)?,
                        rejection_kind: row.get(2)?,
                        rejection_parked_at: row.get(3)?,
                        acquisition_channel: row.get(4)?,
                        item_id: row.get(5)?,
                        document_row_id: row.get(6)?,
                        money_source_id: row.get(7)?,
                        money_source_candidate_id: row.get(8)?,
                        attention_parked_reason: row.get(9)?,
                        active_pipeline: row.get(10)?,
                        candidate_status: row.get(11)?,
                        latest_parse_blocked: row.get(12)?,
                        has_open_review: row.get(13)?,
                        document_is_ready: row.get(14)?,
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
                    if item.source_document_id.is_some() {
                        if item.document_row_id.is_none() {
                            return Err(io::Error::other(
                                "captured intake item has no source document",
                            )
                            .into());
                        }
                        if item.active_pipeline {
                            all_terminal = false;
                            break;
                        }
                        if matches!(
                            item.downstream_state(),
                            DownstreamState::Ready | DownstreamState::Actionable
                        ) {
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
}

/// One batch item plus the document state its terminal decision needs, read in
/// the same statement as the item list.
struct BatchItem {
    capture_outcome: String,
    source_document_id: Option<String>,
    rejection_kind: Option<String>,
    rejection_parked_at: Option<String>,
    acquisition_channel: String,
    item_id: String,
    document_row_id: Option<String>,
    money_source_id: Option<String>,
    money_source_candidate_id: Option<String>,
    attention_parked_reason: Option<String>,
    active_pipeline: bool,
    candidate_status: Option<String>,
    latest_parse_blocked: Option<String>,
    has_open_review: bool,
    document_is_ready: bool,
}

impl BatchItem {
    /// The downstream state this item reports, mirroring the predicate used by
    /// `derive_ready`: a parked statement-password document and a kept-unassigned
    /// candidate are parked; a blocked parse, open review work, or an unassigned
    /// document needs attention; an assigned document is ready only when its
    /// reconcile succeeded and no pipeline job or review item is open.
    fn downstream_state(&self) -> DownstreamState {
        if self.attention_parked_reason.as_deref() == Some("statement_password") {
            return DownstreamState::Other;
        }
        if self.money_source_candidate_id.is_some() {
            return if self.candidate_status.as_deref() == Some("pending") {
                DownstreamState::Actionable
            } else {
                DownstreamState::Other
            };
        }
        if self.latest_parse_blocked.is_some() || self.has_open_review {
            return DownstreamState::Actionable;
        }
        if self.money_source_id.is_some() {
            return if self.document_is_ready {
                DownstreamState::Ready
            } else {
                DownstreamState::Actionable
            };
        }
        DownstreamState::Actionable
    }
}
