use super::*;

pub(super) fn statement_password_state_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StatementPasswordState> {
    let status = match row.get::<_, String>(2)?.as_str() {
        "pending_delete" => StatementPasswordStatus::PendingDelete,
        "pending_save" => StatementPasswordStatus::PendingSave,
        "saved" => StatementPasswordStatus::Saved,
        _ => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid statement password status",
                )
                .into(),
            ));
        }
    };
    Ok(StatementPasswordState {
        money_source_id: row.get(0)?,
        secret_storage_key: row.get(1)?,
        status,
    })
}

pub(super) fn update_statement_password_status(
    connection: &Connection,
    money_source_id: &str,
    expected_status: &str,
    next_status: &str,
) -> StoreResult<()> {
    let changed = connection.execute(
        "UPDATE statement_secret_refs SET status = ?1, updated_at = CURRENT_TIMESTAMP \
         WHERE money_source_id = ?2 AND status = ?3",
        params![next_status, money_source_id, expected_status],
    )?;
    if changed != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "statement password state changed",
        )
        .into());
    }
    Ok(())
}

pub(super) fn source_document_from_row(row: &Row<'_>) -> rusqlite::Result<SourceDocumentView> {
    let byte_size: i64 = row.get(6)?;
    Ok(SourceDocumentView {
        byte_size: u64::try_from(byte_size).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Integer,
                Box::new(error),
            )
        })?,
        document_id: row.get(0)?,
        money_source_id: row.get(1)?,
        file_sha256: row.get(2)?,
        semantic_document_key: row.get(3)?,
        original_filename: row.get(4)?,
        mime_type: row.get(5)?,
        encrypted_locator: row.get(7)?,
        file_state: row.get(8)?,
        received_at: row.get(9)?,
    })
}

#[derive(Debug)]
pub(super) struct ReviewRelationshipRecord {
    pub(super) account_balance_delta: String,
    pub(super) currency: String,
    pub(super) event_type: Option<String>,
    pub(super) record_id: String,
    pub(super) review_item_id: Option<String>,
}

#[derive(Debug)]
pub(super) struct StoredReviewRelationship {
    pub(super) allocation_value: String,
    pub(super) event_type: String,
    pub(super) first_record_id: String,
    pub(super) first_review_item_id: String,
    pub(super) id: String,
    pub(super) second_record_id: String,
    pub(super) second_review_item_id: String,
    pub(super) status: String,
    pub(super) unit: String,
}

impl StoredReviewRelationship {
    pub(super) fn record_ids(&self) -> Vec<String> {
        vec![self.first_record_id.clone(), self.second_record_id.clone()]
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StoredReviewJobResult {
    pub(super) outcomes: Vec<ReviewBatchGroupOutcome>,
}

pub(super) fn review_item_summary_from_row(row: &Row<'_>) -> rusqlite::Result<ReviewItemSummary> {
    Ok(ReviewItemSummary {
        review_item_id: row.get(0)?,
        record_id: row.get(1)?,
        record_version: row.get(2)?,
        reason_code: row.get(3)?,
        amount_value: row.get(4)?,
        currency: row.get(5)?,
        event_type: row.get(6)?,
        posted_on: row.get(7)?,
        account_label: row.get(8)?,
        record_committed: row.get(9)?,
    })
}

pub(super) fn review_item_detail_from_row(row: &Row<'_>) -> rusqlite::Result<ReviewItemDetail> {
    Ok(ReviewItemDetail {
        review_item_id: row.get(0)?,
        record_id: row.get(1)?,
        record_version: row.get(2)?,
        reason_code: row.get(3)?,
        amount_value: row.get(4)?,
        currency: row.get(5)?,
        event_type: row.get(6)?,
        posted_on: row.get(7)?,
        account_label: row.get(8)?,
        document_label: row.get(9)?,
        source_label: row.get(10)?,
        record_committed: row.get(11)?,
    })
}

pub(super) fn core_review_record_from_row(row: &Row<'_>) -> rusqlite::Result<CoreReviewRecord> {
    Ok(CoreReviewRecord {
        id: row.get(0)?,
        account_id: row.get(1)?,
        account_type: row.get(2)?,
        currency: row.get(3)?,
        posted_on: row.get(4)?,
        account_balance_delta: row.get(5)?,
        instrument_id: row.get(6)?,
    })
}

pub(super) fn relationship_candidate_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<RelationshipCandidateSummary> {
    let amount_value: Option<String> = row.get(4)?;
    let account_balance_delta: Option<String> = row.get(5)?;
    Ok(RelationshipCandidateSummary {
        record_id: row.get(0)?,
        record_version: row.get(1)?,
        event_type: row.get(2)?,
        posted_on: row.get(3)?,
        amount_value: amount_value.or(account_balance_delta).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Null,
                io::Error::new(io::ErrorKind::InvalidData, "candidate amount is missing").into(),
            )
        })?,
        currency: row.get(6)?,
        account_label: row.get(7)?,
    })
}

pub(super) fn review_job_summary_from_row(row: &Row<'_>) -> rusqlite::Result<ReviewJobSummary> {
    let status = match row.get::<_, String>(1)?.as_str() {
        "blocked" => ReviewJobStatus::Blocked,
        "cancelled" => ReviewJobStatus::Cancelled,
        "failed" => ReviewJobStatus::Failed,
        "queued" => ReviewJobStatus::Queued,
        "running" => ReviewJobStatus::Running,
        "succeeded" => ReviewJobStatus::Succeeded,
        _ => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                io::Error::new(io::ErrorKind::InvalidData, "invalid review job status").into(),
            ));
        }
    };
    let result_json: Option<String> = row.get(4)?;
    let outcomes = match result_json {
        Some(result_json) => {
            serde_json::from_str::<StoredReviewJobResult>(&result_json)
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .outcomes
        }
        None => Vec::new(),
    };
    Ok(ReviewJobSummary {
        job_id: row.get(0)?,
        status,
        created_at: row.get(2)?,
        finished_at: row.get(3)?,
        outcomes,
    })
}

pub(super) fn open_review_record_for_relationship(
    transaction: &rusqlite::Transaction<'_>,
    review_item_id: &str,
    expected_record_version: i64,
) -> StoreResult<Option<ReviewRelationshipRecord>> {
    let record = transaction
        .query_row(
            "SELECT external_records.id, external_records.event_type, \
                    external_records.account_balance_delta, external_records.currency \
             FROM review_items \
             JOIN external_records ON external_records.id = review_items.external_record_id \
             WHERE review_items.id = ?1 AND review_items.status = 'open' \
               AND external_records.version = ?2 \
               AND external_records.status IN ('staged', 'review')",
            params![review_item_id, expected_record_version],
            |row| {
                Ok(ReviewRelationshipRecord {
                    record_id: row.get(0)?,
                    event_type: row.get(1)?,
                    account_balance_delta: row.get(2)?,
                    currency: row.get(3)?,
                    review_item_id: Some(review_item_id.to_owned()),
                })
            },
        )
        .optional()?;
    Ok(record)
}

pub(super) fn open_review_record_by_id(
    transaction: &rusqlite::Transaction<'_>,
    record_id: &str,
    expected_record_version: i64,
) -> StoreResult<Option<ReviewRelationshipRecord>> {
    let record = transaction
        .query_row(
            "SELECT external_records.id, external_records.event_type, \
                    external_records.account_balance_delta, external_records.currency, review_items.id \
             FROM external_records \
             LEFT JOIN review_items ON review_items.external_record_id = external_records.id \
                AND review_items.status = 'open' \
             WHERE external_records.id = ?1 AND external_records.version = ?2 \
               AND external_records.status IN ('staged', 'review') \
             ORDER BY review_items.id LIMIT 1",
            params![record_id, expected_record_version],
            |row| {
                Ok(ReviewRelationshipRecord {
                    record_id: row.get(0)?,
                    event_type: row.get(1)?,
                    account_balance_delta: row.get(2)?,
                    currency: row.get(3)?,
                    review_item_id: row.get(4)?,
                })
            },
        )
        .optional()?;
    Ok(record)
}

pub(super) fn ordered_relationship_records<'a>(
    first: &'a ReviewRelationshipRecord,
    second: &'a ReviewRelationshipRecord,
) -> (&'a ReviewRelationshipRecord, &'a ReviewRelationshipRecord) {
    if first.record_id < second.record_id {
        (first, second)
    } else {
        (second, first)
    }
}

pub(super) fn same_record_ids(record_ids: &[String], first: &str, second: &str) -> bool {
    if record_ids.len() != 2 {
        return false;
    }
    let mut expected = [first, second];
    expected.sort_unstable();
    let mut actual = [record_ids[0].as_str(), record_ids[1].as_str()];
    actual.sort_unstable();
    actual == expected
}

pub(super) fn review_conflict(reason: &'static str) -> ReviewMutationOutcome {
    ReviewMutationOutcome {
        reason: Some(reason),
        record_version: None,
        review_item_id: None,
        status: ReviewMutationStatus::Conflict,
    }
}
