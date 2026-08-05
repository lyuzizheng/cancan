#![cfg_attr(not(test), expect(dead_code, reason = "repository API checkpoint"))]

use super::*;

mod candidates;
#[cfg(test)]
pub(crate) use candidates::{MoneySourceCandidateInput, MoneySourceCandidateScope};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IntakeAcquisitionChannel {
    ExplicitHandoff,
    Gmail,
    LocalInbox,
}

impl IntakeAcquisitionChannel {
    fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitHandoff => "explicit_handoff",
            Self::Gmail => "gmail",
            Self::LocalInbox => "local_inbox",
        }
    }
}

pub(crate) struct IntakeBatchItemInput<'a> {
    pub(crate) id: &'a str,
    pub(crate) safe_input_label: &'a str,
    pub(crate) acquisition_input_key: Option<&'a str>,
    pub(crate) acquisition_input_version: Option<&'a str>,
    pub(crate) retry_of_batch_item_id: Option<&'a str>,
}

pub(crate) struct IntakeBatchInput<'a> {
    pub(crate) id: &'a str,
    pub(crate) acquisition_channel: IntakeAcquisitionChannel,
    pub(crate) items: &'a [IntakeBatchItemInput<'a>],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IntakeRejectionKind {
    BackgroundActionRequired,
    VisibleReceipt,
}

impl IntakeRejectionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::BackgroundActionRequired => "background_action_required",
            Self::VisibleReceipt => "visible_receipt",
        }
    }
}

pub(crate) enum IntakeItemFinalization<'a> {
    Suppressed {
        code: &'a str,
    },
    Rejected {
        code: &'a str,
        kind: IntakeRejectionKind,
        parked: bool,
    },
}

impl ManualImportStore {
    pub(crate) fn create_intake_batch(&mut self, input: &IntakeBatchInput<'_>) -> StoreResult<()> {
        validate_batch(input)?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO intake_batches( \
               id, acquisition_channel, opened_at, sealed_at \
             ) VALUES (?1, ?2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            params![input.id, input.acquisition_channel.as_str()],
        )?;
        for (ordinal, item) in input.items.iter().enumerate() {
            let retry_state = item
                .retry_of_batch_item_id
                .map(|retry_id| retryable_rejection(&transaction, retry_id))
                .transpose()?;
            transaction.execute(
                "INSERT INTO intake_batch_items( \
                   id, intake_batch_id, input_ordinal, safe_input_label, capture_outcome, \
                   acquisition_input_key, acquisition_input_version, retry_of_batch_item_id \
                 ) VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7)",
                params![
                    item.id,
                    input.id,
                    i64::try_from(ordinal)?,
                    item.safe_input_label,
                    item.acquisition_input_key,
                    item.acquisition_input_version,
                    item.retry_of_batch_item_id,
                ],
            )?;
            if let Some((retry_id, IntakeRejectionKind::BackgroundActionRequired)) =
                item.retry_of_batch_item_id.zip(retry_state)
            {
                let changed = resolve_rejection(&transaction, retry_id, item.id)?;
                if changed != 1 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "intake retry no longer matches an active rejection",
                    )
                    .into());
                }
            }
            if input.acquisition_channel == IntakeAcquisitionChannel::LocalInbox {
                resolve_matching_local_inbox_rejections(
                    &transaction,
                    item.id,
                    item.acquisition_input_key
                        .expect("validated Local Inbox key"),
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn finalize_intake_batch_item(
        &mut self,
        item_id: &str,
        finalization: IntakeItemFinalization<'_>,
    ) -> StoreResult<()> {
        validate_identifier(item_id, "intake item id")?;
        let (outcome, rejection_kind, rejection_code, parked) = match finalization {
            IntakeItemFinalization::Suppressed { code } => {
                validate_stable_code(code)?;
                ("suppressed", None, Some(code), false)
            }
            IntakeItemFinalization::Rejected { code, kind, parked } => {
                validate_stable_code(code)?;
                if parked && kind != IntakeRejectionKind::BackgroundActionRequired {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "only a background rejection can be parked",
                    )
                    .into());
                }
                ("rejected", Some(kind.as_str()), Some(code), parked)
            }
        };
        let changed = self.connection.execute(
            "UPDATE intake_batch_items \
             SET capture_outcome = ?1, rejection_kind = ?2, rejection_code = ?3, \
                 rejection_parked_at = CASE WHEN ?4 THEN CURRENT_TIMESTAMP ELSE NULL END, \
                 finalized_at = CURRENT_TIMESTAMP \
             WHERE id = ?5 AND capture_outcome = 'pending' AND finalized_at IS NULL",
            params![outcome, rejection_kind, rejection_code, parked, item_id,],
        )?;
        if changed != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "intake item is missing or already finalized",
            )
            .into());
        }
        Ok(())
    }

    pub(crate) fn persist_captured_intake_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
        stored: &StoredFile,
        intake_item_id: &str,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        validate_identifier(intake_item_id, "intake item id")?;
        Ok(persist_import(
            &mut self.connection,
            input,
            stored,
            None,
            Some(intake_item_id),
        )?)
    }

    #[cfg(test)]
    pub(crate) fn persist_captured_import(
        &mut self,
        input: &SourceDocumentImport<'_>,
        stored: &StoredFile,
        restore_deleted_document_id: Option<&str>,
    ) -> StoreResult<SourceDocumentImportOutcome> {
        Ok(persist_import(
            &mut self.connection,
            input,
            stored,
            restore_deleted_document_id,
            None,
        )?)
    }

    pub(crate) fn intake_source_capture_plan(
        &mut self,
        input: &SourceDocumentImport<'_>,
        source: &PreparedSource,
        intake_item_id: &str,
        automatic_discovery: bool,
    ) -> StoreResult<SourceCapturePlan> {
        validate_identifier(intake_item_id, "intake item id")?;
        let plan = self.source_capture_plan(input, source, None)?;
        let SourceCapturePlan::RestoreConfirmationRequired(outcome) = &plan else {
            return Ok(plan);
        };
        let transaction = self.connection.transaction()?;
        let exact = find_exact_document(&transaction, source.file_sha256())?;
        if !exact.as_ref().is_some_and(|document| {
            document.document_id == outcome.document_id && document.file_state == "deleted"
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "restore candidate changed before receipt finalization",
            )
            .into());
        }
        if automatic_discovery {
            let changed = transaction.execute(
                "UPDATE intake_batch_items \
                 SET capture_outcome = 'suppressed', rejection_kind = NULL, \
                     rejection_code = 'tombstone_suppressed', rejection_parked_at = NULL, \
                     finalized_at = CURRENT_TIMESTAMP \
                 WHERE id = ?1 AND capture_outcome = 'pending' AND finalized_at IS NULL",
                [intake_item_id],
            )?;
            if changed != 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "intake item is missing or already finalized",
                )
                .into());
            }
        } else {
            finalize_document_intake_item(
                &transaction,
                intake_item_id,
                &outcome.document_id,
                SourceDocumentImportStatus::RestoreConfirmationRequired,
            )?;
        }
        transaction.commit()?;
        let mut plan = plan;
        if let SourceCapturePlan::RestoreConfirmationRequired(outcome) = &mut plan {
            outcome.intake_item_id = Some(intake_item_id.to_owned());
        }
        Ok(plan)
    }

    pub(super) fn recover_interrupted_intake_items(&mut self) -> StoreResult<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE intake_batch_items \
             SET capture_outcome = 'rejected', \
                 rejection_kind = 'background_action_required', \
                 rejection_code = 'handoff_interrupted', \
                 rejection_parked_at = CURRENT_TIMESTAMP, finalized_at = CURRENT_TIMESTAMP \
             WHERE capture_outcome = 'pending' \
               AND intake_batch_id IN ( \
                 SELECT id FROM intake_batches WHERE acquisition_channel = 'explicit_handoff' \
               )",
            [],
        )?;
        transaction.execute(
            "UPDATE intake_batch_items \
             SET capture_outcome = 'suppressed', rejection_code = 'discovery_retry', \
                 finalized_at = CURRENT_TIMESTAMP \
             WHERE capture_outcome = 'pending' \
               AND intake_batch_id IN ( \
                 SELECT id FROM intake_batches WHERE acquisition_channel <> 'explicit_handoff' \
               )",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn recover_pending_local_inbox_items(&mut self, batch_id: &str) -> StoreResult<()> {
        validate_identifier(batch_id, "intake batch id")?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE intake_batch_items \
             SET capture_outcome = 'suppressed', rejection_code = 'discovery_retry', \
                 finalized_at = CURRENT_TIMESTAMP \
             WHERE intake_batch_id = ?1 AND capture_outcome = 'pending' \
               AND intake_batch_id IN ( \
                 SELECT id FROM intake_batches WHERE acquisition_channel = 'local_inbox' \
               )",
            [batch_id],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

pub(super) fn finalize_document_intake_item(
    transaction: &Transaction<'_>,
    item_id: &str,
    document_id: &str,
    status: SourceDocumentImportStatus,
) -> rusqlite::Result<()> {
    let outcome = match status {
        SourceDocumentImportStatus::Imported | SourceDocumentImportStatus::Restored => "captured",
        SourceDocumentImportStatus::AlreadyPresent => "already_present",
        SourceDocumentImportStatus::RestoreConfirmationRequired => "restore_confirmation_required",
    };
    let changed = transaction.execute(
        "UPDATE intake_batch_items \
         SET capture_outcome = ?1, source_document_id = ?2, finalized_at = CURRENT_TIMESTAMP \
         WHERE id = ?3 AND capture_outcome = 'pending' AND finalized_at IS NULL",
        params![outcome, document_id, item_id],
    )?;
    if changed != 1 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

fn validate_batch(input: &IntakeBatchInput<'_>) -> StoreResult<()> {
    validate_identifier(input.id, "intake batch id")?;
    if input.items.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "intake batch must contain at least one item",
        )
        .into());
    }
    let mut ids = HashSet::new();
    for item in input.items {
        validate_identifier(item.id, "intake item id")?;
        if !ids.insert(item.id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "intake item ids must be unique within a batch",
            )
            .into());
        }
        validate_safe_input_label(item.safe_input_label)?;
        if let Some(retry_id) = item.retry_of_batch_item_id {
            validate_identifier(retry_id, "retry intake item id")?;
            if retry_id == item.id {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "an intake item cannot retry itself",
                )
                .into());
            }
        }
        match input.acquisition_channel {
            IntakeAcquisitionChannel::LocalInbox => {
                let (Some(key), Some(version)) =
                    (item.acquisition_input_key, item.acquisition_input_version)
                else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Local Inbox intake requires correlation tokens",
                    )
                    .into());
                };
                if !valid_lower_hex(key, 64)
                    || version.len() != 67
                    || !version.starts_with("v1:")
                    || !valid_lower_hex(&version[3..], 64)
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "invalid Local Inbox correlation tokens",
                    )
                    .into());
                }
            }
            IntakeAcquisitionChannel::ExplicitHandoff | IntakeAcquisitionChannel::Gmail => {
                if item.acquisition_input_key.is_some() || item.acquisition_input_version.is_some()
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "this intake channel does not accept scanner correlation tokens",
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

fn retryable_rejection(
    transaction: &Transaction<'_>,
    item_id: &str,
) -> StoreResult<IntakeRejectionKind> {
    let state = transaction
        .query_row(
            "SELECT capture_outcome, rejection_kind, rejection_resolved_at \
             FROM intake_batch_items WHERE id = ?1",
            [item_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;
    match state {
        Some((outcome, Some(kind), resolved_at)) if outcome == "rejected" => match kind.as_str() {
            "background_action_required" if resolved_at.is_none() => {
                Ok(IntakeRejectionKind::BackgroundActionRequired)
            }
            "visible_receipt" => Ok(IntakeRejectionKind::VisibleReceipt),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "intake rejection is already resolved",
            )
            .into()),
        },
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "retry target is not a rejected intake item",
        )
        .into()),
    }
}

fn resolve_rejection(
    transaction: &Transaction<'_>,
    rejected_item_id: &str,
    replacement_item_id: &str,
) -> rusqlite::Result<usize> {
    transaction.execute(
        "UPDATE intake_batch_items \
         SET rejection_resolved_at = CURRENT_TIMESTAMP, \
             rejection_resolution_kind = 'superseded_by_new_attempt', \
             resolved_by_batch_item_id = ?1 \
         WHERE id = ?2 AND capture_outcome = 'rejected' \
           AND rejection_kind = 'background_action_required' \
           AND rejection_resolved_at IS NULL",
        params![replacement_item_id, rejected_item_id],
    )
}

fn resolve_matching_local_inbox_rejections(
    transaction: &Transaction<'_>,
    replacement_item_id: &str,
    entry_key: &str,
) -> rusqlite::Result<()> {
    transaction.execute(
        "UPDATE intake_batch_items \
         SET rejection_resolved_at = CURRENT_TIMESTAMP, \
             rejection_resolution_kind = 'superseded_by_new_attempt', \
             resolved_by_batch_item_id = ?1 \
         WHERE id <> ?1 AND capture_outcome = 'rejected' \
           AND rejection_kind = 'background_action_required' \
           AND rejection_resolved_at IS NULL AND acquisition_input_key = ?2 \
           AND intake_batch_id IN ( \
             SELECT id FROM intake_batches WHERE acquisition_channel = 'local_inbox' \
           )",
        params![replacement_item_id, entry_key],
    )?;
    Ok(())
}

pub(crate) fn validate_identifier(value: &str, name: &str) -> StoreResult<()> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {name}")).into());
    }
    Ok(())
}

fn validate_safe_input_label(value: &str) -> StoreResult<()> {
    if value.is_empty()
        || value.len() > 240
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
        || value.chars().any(is_control_or_bidi)
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid safe input label").into());
    }
    Ok(())
}

pub(crate) fn bounded_safe_input_label(value: &str) -> String {
    let mut label = value
        .chars()
        .filter(|character| {
            !is_control_or_bidi(*character) && *character != '/' && *character != '\\'
        })
        .collect::<String>();
    while label.len() > 240 {
        label.pop();
    }
    if label.is_empty() {
        "File".to_owned()
    } else {
        label
    }
}

fn is_control_or_bidi(value: char) -> bool {
    value.is_control()
        || matches!(
            value,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

fn validate_stable_code(value: &str) -> StoreResult<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "invalid intake reason code").into(),
        );
    }
    Ok(())
}

fn valid_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
