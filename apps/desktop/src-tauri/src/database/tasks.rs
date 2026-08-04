use super::*;

#[cfg(test)]
use super::intake::validate_identifier;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RawTask {
    pub(crate) title: String,
    pub(crate) timestamp: String,
    pub(crate) kind: RawTaskKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RawTaskKind {
    Processing {
        document_id: String,
    },
    PasswordNeeded {
        document_id: String,
        money_source_id: String,
    },
    NewSource {
        candidate_id: String,
    },
    NeedsReview {
        document_id: String,
    },
    RestoreSourceFile {
        intake_item_id: String,
        document_id: String,
    },
    InboxFileCouldNotBeAdded {
        intake_item_id: String,
    },
    ImportInterrupted {
        intake_item_id: String,
    },
    NeedsAttention {
        document_id: String,
    },
    FileNotAdded {
        intake_item_id: String,
    },
    AlreadyInCancan {
        intake_item_id: String,
    },
    SourceFileRestored {
        intake_item_id: String,
    },
    SourceFileLeftDeleted {
        intake_item_id: String,
    },
    Ready {
        intake_item_id: String,
    },
    SourceUnassigned {
        candidate_id: String,
    },
    PasswordParked {
        document_id: String,
        money_source_id: String,
    },
    InboxFileParked {
        intake_item_id: String,
    },
}

enum DownstreamState {
    Ready,
    Actionable,
    Other,
}

impl ManualImportStore {
    pub(crate) fn derive_task_rows(
        &self,
        as_of: Option<&str>,
        include_parked: bool,
    ) -> StoreResult<Vec<RawTask>> {
        let mut rows = Vec::new();
        rows.extend(self.derive_processing_tasks()?);
        rows.extend(self.derive_password_needed_tasks()?);
        rows.extend(self.derive_new_source_tasks()?);
        rows.extend(self.derive_needs_review_tasks()?);
        rows.extend(self.derive_restore_source_tasks()?);
        rows.extend(self.derive_inbox_tasks()?);
        rows.extend(self.derive_needs_attention_tasks()?);
        rows.extend(self.derive_recently_completed_tasks(as_of)?);
        if include_parked {
            rows.extend(self.derive_parked_tasks()?);
        }
        Ok(rows)
    }

    fn derive_processing_tasks(&self) -> StoreResult<Vec<RawTask>> {
        let mut statement = self.connection.prepare(
            "SELECT sd.id, sd.original_filename, MAX(j.updated_at) \
             FROM source_documents sd \
             JOIN jobs j ON j.related_source_document_id = sd.id \
             WHERE sd.file_state = 'available' \
               AND sd.money_source_id IS NOT NULL \
               AND sd.attention_parked_reason IS NULL \
               AND j.job_type IN ('parse_document', 'reconcile_document') \
               AND j.status IN ('queued', 'running') \
             GROUP BY sd.id, sd.original_filename",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(RawTask {
                    title: row.get(1)?,
                    timestamp: row.get(2)?,
                    kind: RawTaskKind::Processing {
                        document_id: row.get(0)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn derive_password_needed_tasks(&self) -> StoreResult<Vec<RawTask>> {
        let mut statement = self.connection.prepare(
            "SELECT sd.id, sd.money_source_id, sd.original_filename, j.updated_at \
             FROM source_documents sd \
             JOIN jobs j ON j.id = ( \
               SELECT id FROM jobs \
               WHERE related_source_document_id = sd.id \
                 AND job_type = 'parse_document' \
               ORDER BY created_at DESC, id DESC LIMIT 1 \
             ) \
             WHERE sd.file_state = 'available' \
               AND sd.money_source_id IS NOT NULL \
               AND sd.attention_parked_reason IS NULL \
               AND j.status = 'blocked' \
               AND j.blocked_reason = 'password_required'",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(RawTask {
                    title: row.get(2)?,
                    timestamp: row.get(3)?,
                    kind: RawTaskKind::PasswordNeeded {
                        document_id: row.get(0)?,
                        money_source_id: row.get(1)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn derive_new_source_tasks(&self) -> StoreResult<Vec<RawTask>> {
        let mut statement = self.connection.prepare(
            "SELECT c.id, MAX(sd.original_filename), c.updated_at \
             FROM money_source_candidates c \
             JOIN source_documents sd ON sd.money_source_candidate_id = c.id \
             WHERE c.status = 'pending' \
               AND sd.money_source_id IS NULL \
               AND NOT ( \
                 sd.attention_parked_reason = 'source_confirmation' \
                 AND sd.attention_parked_at IS NOT NULL \
               ) \
             GROUP BY c.id, c.updated_at",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(RawTask {
                    title: row.get(1)?,
                    timestamp: row.get(2)?,
                    kind: RawTaskKind::NewSource {
                        candidate_id: row.get(0)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn derive_needs_review_tasks(&self) -> StoreResult<Vec<RawTask>> {
        let mut statement = self.connection.prepare(
            "SELECT sd.id, sd.original_filename, MAX(ri.created_at) \
             FROM review_items ri \
             JOIN external_records er ON er.id = ri.external_record_id \
             JOIN source_documents sd ON sd.id = er.source_document_id \
             WHERE ri.status = 'open' \
               AND er.status IN ('staged', 'review') \
             GROUP BY sd.id, sd.original_filename",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(RawTask {
                    title: row.get(1)?,
                    timestamp: row.get(2)?,
                    kind: RawTaskKind::NeedsReview {
                        document_id: row.get(0)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn derive_restore_source_tasks(&self) -> StoreResult<Vec<RawTask>> {
        let mut statement = self.connection.prepare(
            "SELECT i.id, i.source_document_id, i.safe_input_label, i.finalized_at \
             FROM intake_batch_items i \
             JOIN source_documents sd ON sd.id = i.source_document_id \
             WHERE i.capture_outcome = 'restore_confirmation_required' \
               AND sd.file_state = 'deleted' \
               AND i.finalized_at IS NOT NULL \
               AND NOT EXISTS ( \
                 SELECT 1 FROM audit_log al \
                 WHERE al.entity_type = 'source_document' \
                   AND al.entity_id = sd.id \
                   AND al.action IN ('source_file_restored', 'source_file_restore_declined') \
               )",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(RawTask {
                    title: row.get(2)?,
                    timestamp: row.get(3)?,
                    kind: RawTaskKind::RestoreSourceFile {
                        intake_item_id: row.get(0)?,
                        document_id: row.get(1)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn derive_inbox_tasks(&self) -> StoreResult<Vec<RawTask>> {
        let mut statement = self.connection.prepare(
            "SELECT i.id, i.safe_input_label, i.finalized_at, b.acquisition_channel, i.rejection_code \
             FROM intake_batch_items i \
             JOIN intake_batches b ON b.id = i.intake_batch_id \
             WHERE i.capture_outcome = 'rejected' \
               AND i.rejection_kind = 'background_action_required' \
               AND i.rejection_resolved_at IS NULL \
               AND ( \
                 (b.acquisition_channel = 'local_inbox' AND i.rejection_code != 'handoff_interrupted' AND i.rejection_parked_at IS NULL) \
                 OR (b.acquisition_channel = 'explicit_handoff' AND i.rejection_code = 'handoff_interrupted') \
               )",
        )?;
        let rows = statement
            .query_map([], |row| {
                let channel: String = row.get(3)?;
                let code: String = row.get(4)?;
                let kind = if channel == "explicit_handoff" && code == "handoff_interrupted" {
                    RawTaskKind::ImportInterrupted {
                        intake_item_id: row.get(0)?,
                    }
                } else {
                    RawTaskKind::InboxFileCouldNotBeAdded {
                        intake_item_id: row.get(0)?,
                    }
                };
                Ok(RawTask {
                    title: row.get(1)?,
                    timestamp: row.get(2)?,
                    kind,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn derive_needs_attention_tasks(&self) -> StoreResult<Vec<RawTask>> {
        let mut statement = self.connection.prepare(
            "SELECT sd.id, sd.original_filename, j.updated_at, j.blocked_reason \
             FROM source_documents sd \
             JOIN jobs j ON j.id = ( \
               SELECT id FROM jobs \
               WHERE related_source_document_id = sd.id \
                 AND job_type = 'parse_document' \
               ORDER BY created_at DESC, id DESC LIMIT 1 \
             ) \
             WHERE sd.file_state = 'available' \
               AND sd.money_source_id IS NULL \
               AND sd.money_source_candidate_id IS NULL \
               AND j.status IN ('blocked', 'failed') \
               AND j.blocked_reason IS NOT NULL \
               AND j.blocked_reason != 'password_required' \
               AND NOT EXISTS ( \
                 SELECT 1 FROM review_items ri \
                 JOIN external_records er ON er.id = ri.external_record_id \
                 WHERE er.source_document_id = sd.id \
                   AND ri.status = 'open' \
                   AND er.status IN ('staged', 'review') \
               )",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(RawTask {
                    title: row.get(1)?,
                    timestamp: row.get(2)?,
                    kind: RawTaskKind::NeedsAttention {
                        document_id: row.get(0)?,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn derive_recently_completed_tasks(&self, as_of: Option<&str>) -> StoreResult<Vec<RawTask>> {
        let cutoff = as_of;
        let mut rows = Vec::new();

        let mut statement = self.connection.prepare(
            "SELECT id, safe_input_label, finalized_at \
             FROM intake_batch_items \
             WHERE capture_outcome = 'rejected' \
               AND rejection_kind = 'visible_receipt' \
               AND finalized_at >= datetime(COALESCE(?1,'now'), '-168 hours')",
        )?;
        rows.extend(
            statement
                .query_map([cutoff], |row| {
                    Ok(RawTask {
                        title: row.get(1)?,
                        timestamp: row.get(2)?,
                        kind: RawTaskKind::FileNotAdded {
                            intake_item_id: row.get(0)?,
                        },
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
        );

        let mut statement = self.connection.prepare(
            "SELECT id, safe_input_label, finalized_at \
             FROM intake_batch_items \
             WHERE capture_outcome = 'already_present' \
               AND finalized_at >= datetime(COALESCE(?1,'now'), '-168 hours')",
        )?;
        rows.extend(
            statement
                .query_map([cutoff], |row| {
                    Ok(RawTask {
                        title: row.get(1)?,
                        timestamp: row.get(2)?,
                        kind: RawTaskKind::AlreadyInCancan {
                            intake_item_id: row.get(0)?,
                        },
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
        );

        let mut statement = self.connection.prepare(
            "SELECT i.id, i.safe_input_label, al.action, MAX(al.created_at) \
             FROM intake_batch_items i \
             JOIN source_documents sd ON sd.id = i.source_document_id \
             JOIN audit_log al ON al.entity_type = 'source_document' \
               AND al.entity_id = sd.id \
               AND al.action IN ('source_file_restored', 'source_file_restore_declined') \
             WHERE i.capture_outcome = 'restore_confirmation_required' \
               AND al.created_at >= datetime(COALESCE(?1,'now'), '-168 hours') \
             GROUP BY i.id, i.safe_input_label, al.action",
        )?;
        rows.extend(
            statement
                .query_map([cutoff], |row| {
                    let action: String = row.get(2)?;
                    let kind = if action == "source_file_restored" {
                        RawTaskKind::SourceFileRestored {
                            intake_item_id: row.get(0)?,
                        }
                    } else {
                        RawTaskKind::SourceFileLeftDeleted {
                            intake_item_id: row.get(0)?,
                        }
                    };
                    Ok(RawTask {
                        title: row.get(1)?,
                        timestamp: row.get(3)?,
                        kind,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
        );

        let mut statement = self.connection.prepare(
            "SELECT i.id, i.safe_input_label, i.finalized_at \
             FROM intake_batch_items i \
             JOIN source_documents sd ON sd.id = i.source_document_id \
             WHERE i.capture_outcome = 'captured' \
               AND i.finalized_at >= datetime(COALESCE(?1,'now'), '-168 hours') \
               AND sd.file_state = 'available' \
               AND sd.money_source_id IS NOT NULL \
               AND sd.money_source_candidate_id IS NULL \
               AND sd.attention_parked_reason IS NULL \
               AND NOT EXISTS ( \
                 SELECT 1 FROM jobs j \
                 WHERE j.related_source_document_id = sd.id \
                   AND j.job_type IN ('parse_document', 'reconcile_document') \
                   AND j.status IN ('queued', 'running') \
               ) \
               AND NOT EXISTS ( \
                 SELECT 1 FROM review_items ri \
                 JOIN external_records er ON er.id = ri.external_record_id \
                 WHERE er.source_document_id = sd.id \
                   AND ri.status = 'open' \
                   AND er.status IN ('staged', 'review') \
               )",
        )?;
        rows.extend(
            statement
                .query_map([cutoff], |row| {
                    Ok(RawTask {
                        title: row.get(1)?,
                        timestamp: row.get(2)?,
                        kind: RawTaskKind::Ready {
                            intake_item_id: row.get(0)?,
                        },
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
        );

        Ok(rows)
    }

    fn derive_parked_tasks(&self) -> StoreResult<Vec<RawTask>> {
        let mut rows = Vec::new();

        let mut statement = self.connection.prepare(
            "SELECT c.id, MAX(sd.original_filename), MAX(sd.attention_parked_at) \
             FROM money_source_candidates c \
             JOIN source_documents sd ON sd.money_source_candidate_id = c.id \
             WHERE c.status = 'kept_unassigned' \
               AND sd.money_source_id IS NULL \
               AND sd.attention_parked_reason = 'source_confirmation' \
               AND sd.attention_parked_at IS NOT NULL \
             GROUP BY c.id",
        )?;
        rows.extend(
            statement
                .query_map([], |row| {
                    Ok(RawTask {
                        title: row.get(1)?,
                        timestamp: row.get(2)?,
                        kind: RawTaskKind::SourceUnassigned {
                            candidate_id: row.get(0)?,
                        },
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
        );

        let mut statement = self.connection.prepare(
            "SELECT id, money_source_id, original_filename, attention_parked_at \
             FROM source_documents \
             WHERE file_state = 'available' \
               AND money_source_id IS NOT NULL \
               AND attention_parked_reason = 'statement_password' \
               AND attention_parked_at IS NOT NULL",
        )?;
        rows.extend(
            statement
                .query_map([], |row| {
                    Ok(RawTask {
                        title: row.get(2)?,
                        timestamp: row.get(3)?,
                        kind: RawTaskKind::PasswordParked {
                            document_id: row.get(0)?,
                            money_source_id: row.get(1)?,
                        },
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
        );

        let mut statement = self.connection.prepare(
            "SELECT i.id, i.safe_input_label, i.rejection_parked_at \
             FROM intake_batch_items i \
             JOIN intake_batches b ON b.id = i.intake_batch_id \
             WHERE i.capture_outcome = 'rejected' \
               AND i.rejection_kind = 'background_action_required' \
               AND i.rejection_parked_at IS NOT NULL \
               AND i.rejection_resolved_at IS NULL \
               AND b.acquisition_channel = 'local_inbox'",
        )?;
        rows.extend(
            statement
                .query_map([], |row| {
                    Ok(RawTask {
                        title: row.get(1)?,
                        timestamp: row.get(2)?,
                        kind: RawTaskKind::InboxFileParked {
                            intake_item_id: row.get(0)?,
                        },
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
        );

        Ok(rows)
    }

    pub(crate) fn reconcile_sealed_batches(&mut self, _as_of: Option<&str>) -> StoreResult<()> {
        let batch_ids: Vec<String> = self
            .connection
            .prepare("SELECT id FROM intake_batches WHERE completed_at IS NULL")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for batch_id in batch_ids {
            let transaction = self.connection.transaction()?;
            let completed: Option<Option<String>> = transaction
                .query_row(
                    "SELECT completed_at FROM intake_batches WHERE id = ?1",
                    [&batch_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()?;
            if completed.flatten().is_some() {
                continue;
            }

            let mut statement = transaction.prepare(
                "SELECT capture_outcome, source_document_id, rejection_kind \
                 FROM intake_batch_items \
                 WHERE intake_batch_id = ?1",
            )?;
            struct Item {
                capture_outcome: String,
                source_document_id: Option<String>,
                rejection_kind: Option<String>,
            }
            let items = statement
                .query_map([&batch_id], |row| {
                    Ok(Item {
                        capture_outcome: row.get(0)?,
                        source_document_id: row.get(1)?,
                        rejection_kind: row.get(2)?,
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
                } else if item.capture_outcome == "restore_confirmation_required"
                    || (item.capture_outcome == "rejected"
                        && item.rejection_kind.as_deref() == Some("background_action_required"))
                {
                    any_user_meaningful = true;
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
                 ORDER BY created_at DESC, id DESC LIMIT 1",
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
            return Ok(DownstreamState::Ready);
        }

        // Unassigned and no candidate/job -> needs attention
        Ok(DownstreamState::Actionable)
    }

    #[cfg(test)]
    pub(crate) fn insert_local_inbox_rejected_batch_for_test(
        &mut self,
        batch_id: &str,
        item_id: &str,
        label: &str,
        finalized_at: &str,
        code: &str,
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
               finalized_at \
             ) VALUES (?1, ?2, 0, ?3, 'rejected', 'background_action_required', ?4, ?5, ?6, ?7)",
            params![item_id, batch_id, label, code, key, version, finalized_at],
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

    pub(crate) fn current_timestamp(&self) -> StoreResult<String> {
        let timestamp: String =
            self.connection
                .query_row("SELECT CURRENT_TIMESTAMP", [], |row| row.get(0))?;
        Ok(timestamp)
    }

    pub(crate) fn format_unix_timestamp(&self, seconds: u64) -> StoreResult<String> {
        let timestamp: String = self.connection.query_row(
            "SELECT datetime(?1, 'unixepoch')",
            [seconds.to_string()],
            |row| row.get(0),
        )?;
        Ok(timestamp)
    }
}
