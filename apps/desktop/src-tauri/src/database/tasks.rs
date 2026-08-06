use std::collections::{HashMap, HashSet};

use super::restore_decisions::restore_decision_audit_id;
use super::*;

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

pub(crate) enum DownstreamState {
    Ready,
    Actionable,
    Other,
}

impl ManualImportStore {
    pub(crate) fn derive_task_rows(&self, include_parked: bool) -> StoreResult<Vec<RawTask>> {
        let mut rows = Vec::new();
        rows.extend(self.derive_processing_tasks()?);
        rows.extend(self.derive_password_needed_tasks()?);
        rows.extend(self.derive_new_source_tasks()?);
        rows.extend(self.derive_needs_review_tasks()?);
        rows.extend(self.derive_restore_source_tasks()?);
        rows.extend(self.derive_inbox_tasks()?);
        rows.extend(self.derive_needs_attention_tasks()?);
        rows.extend(self.derive_recently_completed_tasks()?);
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
               ORDER BY created_at DESC, rowid DESC LIMIT 1 \
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
            "SELECT c.id, c.updated_at, \
                 COALESCE( \
                   (SELECT sd.original_filename FROM source_documents sd \
                    WHERE sd.money_source_candidate_id = c.id \
                      AND sd.money_source_id IS NULL \
                    ORDER BY sd.received_at DESC, sd.id DESC LIMIT 1), \
                   'Imported document' \
                 ) \
             FROM money_source_candidates c \
             WHERE c.status = 'pending' \
               AND EXISTS ( \
                 SELECT 1 FROM source_documents sd \
                 WHERE sd.money_source_candidate_id = c.id \
                   AND sd.money_source_id IS NULL \
                   AND NOT ( \
                     sd.attention_parked_reason = 'source_confirmation' \
                     AND sd.attention_parked_at IS NOT NULL \
                   ) \
               ) \
             ORDER BY c.updated_at, c.id",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(RawTask {
                    title: row.get(2)?,
                    timestamp: row.get(1)?,
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
               AND i.finalized_at IS NOT NULL",
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
        let resolved = self.resolved_restore_decision_item_ids()?;
        let rows = rows
            .into_iter()
            .filter(|task| match &task.kind {
                RawTaskKind::RestoreSourceFile { intake_item_id, .. } => {
                    !resolved.contains(intake_item_id)
                }
                _ => true,
            })
            .collect();
        Ok(rows)
    }

    /// Item ids whose restore decision was declined.
    pub(super) fn restore_declined_item_ids(&self) -> StoreResult<HashSet<String>> {
        let (_, declined) = self.restore_decision_item_ids()?;
        Ok(declined)
    }

    fn resolved_restore_decision_item_ids(&self) -> StoreResult<HashSet<String>> {
        let (restored, declined) = self.restore_decision_item_ids()?;
        Ok(restored.union(&declined).cloned().collect())
    }

    /// Per-intake-item restore outcomes recorded in the audit log, as
    /// `(restored, declined)` sets. Restore decisions are recorded in audit_log
    /// with per-intake-item ids (restore_decision_audit_id), so each receipt must
    /// be matched against its own audit rows rather than any restore audit on the
    /// document.
    fn restore_decision_item_ids(&self) -> StoreResult<(HashSet<String>, HashSet<String>)> {
        let mut item_ids: Vec<String> = self
            .connection
            .prepare(
                "SELECT i.id FROM intake_batch_items i \
                 WHERE i.capture_outcome = 'restore_confirmation_required'",
            )?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut restored = HashSet::new();
        let mut declined = HashSet::new();
        if item_ids.is_empty() {
            return Ok((restored, declined));
        }
        let restored_action = "source_file_restored";
        let declined_action = "source_file_restore_declined";
        let mut audit_ids: Vec<String> = Vec::with_capacity(item_ids.len() * 2);
        let mut by_audit_id: HashMap<String, (String, bool)> = HashMap::new();
        for item_id in item_ids.drain(..) {
            for (action, was_restored) in [(restored_action, true), (declined_action, false)] {
                let audit_id = restore_decision_audit_id(&item_id, action);
                by_audit_id.insert(audit_id.clone(), (item_id.clone(), was_restored));
                audit_ids.push(audit_id);
            }
        }
        let mut statement = self.connection.prepare(&format!(
            "SELECT id FROM audit_log \
             WHERE entity_type = 'source_document' \
               AND id IN ({})",
            vec!["?"; audit_ids.len()].join(", ")
        ))?;
        let found: Vec<String> = statement
            .query_map(rusqlite::params_from_iter(audit_ids.iter()), |row| {
                row.get(0)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for audit_id in found {
            if let Some((item_id, was_restored)) = by_audit_id.get(&audit_id) {
                if *was_restored {
                    restored.insert(item_id.clone());
                } else {
                    declined.insert(item_id.clone());
                }
            }
        }
        Ok((restored, declined))
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
        // Covers both failed/blocked parse jobs and failed reconcile jobs: reconcile
        // runs after the money source is assigned, so a failed reconcile must surface
        // even though money_source_id is already set. Parked documents are excluded;
        // they surface in the Parked group instead.
        let mut statement = self.connection.prepare(
            "SELECT sd.id, sd.original_filename, j.updated_at, j.blocked_reason \
             FROM source_documents sd \
             JOIN jobs j ON j.id = ( \
               SELECT id FROM jobs \
               WHERE related_source_document_id = sd.id \
                 AND job_type IN ('parse_document', 'reconcile_document') \
               ORDER BY created_at DESC, rowid DESC LIMIT 1 \
             ) \
             WHERE sd.file_state = 'available' \
               AND sd.money_source_candidate_id IS NULL \
               AND sd.attention_parked_reason IS NULL \
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

    fn derive_recently_completed_tasks(&self) -> StoreResult<Vec<RawTask>> {
        let mut rows = Vec::new();

        let mut statement = self.connection.prepare(
            "SELECT id, safe_input_label, finalized_at \
             FROM intake_batch_items \
             WHERE capture_outcome = 'rejected' \
               AND rejection_kind = 'visible_receipt' \
               AND datetime(finalized_at) >= datetime('now', '-168 hours')",
        )?;
        rows.extend(
            statement
                .query_map([], |row| {
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
               AND datetime(finalized_at) >= datetime('now', '-168 hours')",
        )?;
        rows.extend(
            statement
                .query_map([], |row| {
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

        // Restore receipts are matched against their own per-intake-item audit rows
        // (restore_decision_audit_id). Joining on the source document would let one
        // decision suppress or duplicate every receipt for the same tombstone.
        let mut candidates: Vec<(String, String)> = self
            .connection
            .prepare(
                "SELECT i.id, i.safe_input_label \
                 FROM intake_batch_items i \
                 WHERE i.capture_outcome = 'restore_confirmation_required'",
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        if !candidates.is_empty() {
            let cutoff: String =
                self.connection
                    .query_row("SELECT datetime('now', '-168 hours')", [], |row| row.get(0))?;
            let mut audit_ids: Vec<String> = Vec::with_capacity(candidates.len() * 2);
            let mut by_audit_id: HashMap<String, (String, String)> = HashMap::new();
            for (item_id, label) in candidates.drain(..) {
                for action in ["source_file_restored", "source_file_restore_declined"] {
                    let audit_id = restore_decision_audit_id(&item_id, action);
                    by_audit_id.insert(audit_id.clone(), (item_id.clone(), label.clone()));
                    audit_ids.push(audit_id);
                }
            }
            let mut statement = self.connection.prepare(&format!(
                "SELECT id, action, created_at FROM audit_log \
                 WHERE entity_type = 'source_document' \
                   AND id IN ({})",
                vec!["?"; audit_ids.len()].join(", ")
            ))?;
            let found: Vec<(String, String, String)> = statement
                .query_map(rusqlite::params_from_iter(audit_ids.iter()), |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            drop(statement);
            for (audit_id, action, created_at) in found {
                let Some((item_id, label)) = by_audit_id.get(&audit_id) else {
                    continue;
                };
                if created_at < cutoff {
                    continue;
                }
                let kind = if action == "source_file_restored" {
                    RawTaskKind::SourceFileRestored {
                        intake_item_id: item_id.clone(),
                    }
                } else {
                    RawTaskKind::SourceFileLeftDeleted {
                        intake_item_id: item_id.clone(),
                    }
                };
                rows.push(RawTask {
                    title: label.clone(),
                    timestamp: created_at,
                    kind,
                });
            }
        }

        // Ready receipts: a captured document only becomes Ready once reconcile has
        // succeeded, so the 168-hour retention runs from the reconcile job's
        // finished_at rather than from capture time. Any non-succeeded pipeline job
        // (queued/running/blocked/failed) keeps the document out of Ready.
        let mut statement = self.connection.prepare(
            "SELECT i.id, i.safe_input_label, ready.finished_at \
             FROM intake_batch_items i \
             JOIN source_documents sd ON sd.id = i.source_document_id \
             JOIN ( \
               SELECT related_source_document_id AS document_id, MAX(finished_at) AS finished_at \
               FROM jobs \
               WHERE job_type = 'reconcile_document' AND status = 'succeeded' \
               GROUP BY related_source_document_id \
             ) ready ON ready.document_id = sd.id \
             WHERE i.capture_outcome = 'captured' \
               AND datetime(ready.finished_at) >= datetime('now', '-168 hours') \
               AND sd.file_state = 'available' \
               AND sd.money_source_id IS NOT NULL \
               AND sd.money_source_candidate_id IS NULL \
               AND sd.attention_parked_reason IS NULL \
               AND NOT EXISTS ( \
                 SELECT 1 FROM jobs j \
                 WHERE j.related_source_document_id = sd.id \
                   AND j.job_type IN ('parse_document', 'reconcile_document') \
                   AND j.status != 'succeeded' \
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
                .query_map([], |row| {
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
            "SELECT c.id, MAX(sd.attention_parked_at), \
                 COALESCE( \
                   (SELECT sd.original_filename FROM source_documents sd \
                    WHERE sd.money_source_candidate_id = c.id \
                      AND sd.money_source_id IS NULL \
                      AND sd.attention_parked_reason = 'source_confirmation' \
                      AND sd.attention_parked_at IS NOT NULL \
                    ORDER BY sd.received_at DESC, sd.id DESC LIMIT 1), \
                   'Imported document' \
                 ) \
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
                        title: row.get(2)?,
                        timestamp: row.get(1)?,
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
