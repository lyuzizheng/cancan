use super::*;

/// One slot of the prepared review-batch order. A relationship still needs the
/// two core records the commit will carry; an outcome is complete on its own.
enum PreparedReviewEntry {
    Relationship(StoredReviewRelationship),
    Outcome(ReviewBatchGroupOutcome),
}

impl ManualImportStore {
    /// Prepares every claimed review item with three queries in total — the
    /// selected items' record ids, their relationships, and the relationships'
    /// core records — instead of roughly four queries per item.
    pub(crate) fn prepare_commit_review_groups(
        &self,
        claimed: &ClaimedReviewBatch,
    ) -> StoreResult<(Vec<CommitReviewGroup>, Vec<ReviewBatchGroupOutcome>)> {
        let selected_review_items = claimed
            .review_item_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let record_ids = self.review_item_record_ids(&claimed.review_item_ids)?;
        let mut relationships_by_review_item =
            self.relationships_for_review_items(&claimed.review_item_ids)?;
        let mut seen_relationships = HashSet::new();
        let mut entries = Vec::new();
        for review_item_id in &claimed.review_item_ids {
            let Some(record_id) = record_ids.get(review_item_id) else {
                entries.push(PreparedReviewEntry::Outcome(ReviewBatchGroupOutcome {
                    reason: Some("stale_review_item".to_owned()),
                    record_ids: Vec::new(),
                    status: ReviewBatchGroupStatus::Stale,
                }));
                continue;
            };
            let relationships = relationships_by_review_item
                .remove(review_item_id)
                .unwrap_or_default();
            let accepted = relationships
                .iter()
                .filter(|relationship| relationship.status == "accepted")
                .collect::<Vec<_>>();
            let committed = relationships
                .iter()
                .filter(|relationship| relationship.status == "committed")
                .collect::<Vec<_>>();
            if accepted.len() > 1 {
                entries.push(PreparedReviewEntry::Outcome(ReviewBatchGroupOutcome {
                    reason: Some("ambiguous_relationship".to_owned()),
                    record_ids: vec![record_id.clone()],
                    status: ReviewBatchGroupStatus::StillNeedsReview,
                }));
                continue;
            }
            if let Some(relationship) = committed.first() {
                if seen_relationships.insert(relationship.id.clone()) {
                    entries.push(PreparedReviewEntry::Outcome(ReviewBatchGroupOutcome {
                        reason: None,
                        record_ids: relationship.record_ids(),
                        status: ReviewBatchGroupStatus::AlreadyCommitted,
                    }));
                }
                continue;
            }
            let Some(relationship) = accepted.first() else {
                entries.push(PreparedReviewEntry::Outcome(ReviewBatchGroupOutcome {
                    reason: Some("relationship_not_confirmed".to_owned()),
                    record_ids: vec![record_id.clone()],
                    status: ReviewBatchGroupStatus::StillNeedsReview,
                }));
                continue;
            };
            if !selected_review_items.contains(relationship.first_review_item_id.as_str())
                || !selected_review_items.contains(relationship.second_review_item_id.as_str())
            {
                if seen_relationships.insert(relationship.id.clone()) {
                    entries.push(PreparedReviewEntry::Outcome(ReviewBatchGroupOutcome {
                        reason: Some("relationship_not_selected".to_owned()),
                        record_ids: vec![record_id.clone()],
                        status: ReviewBatchGroupStatus::StillNeedsReview,
                    }));
                }
                continue;
            }
            if !seen_relationships.insert(relationship.id.clone()) {
                continue;
            }
            entries.push(PreparedReviewEntry::Relationship((*relationship).clone()));
        }
        let mut record_ids_to_load = Vec::new();
        for entry in &entries {
            if let PreparedReviewEntry::Relationship(relationship) = entry {
                record_ids_to_load.push(relationship.first_record_id.clone());
                record_ids_to_load.push(relationship.second_record_id.clone());
            }
        }
        let records = self.core_records_by_id(&record_ids_to_load)?;
        let mut groups = Vec::new();
        let mut outcomes = Vec::new();
        for entry in entries {
            match entry {
                PreparedReviewEntry::Relationship(relationship) => {
                    // Two prepared relationships may share one core record (a
                    // record can own more than one review item), so every group
                    // reads its records without consuming them.
                    let first = records.get(&relationship.first_record_id).cloned();
                    let second = records.get(&relationship.second_record_id).cloned();
                    let (Some(first), Some(second)) = (first, second) else {
                        outcomes.push(ReviewBatchGroupOutcome {
                            reason: Some("stale_relationship".to_owned()),
                            record_ids: relationship.record_ids(),
                            status: ReviewBatchGroupStatus::Stale,
                        });
                        continue;
                    };
                    groups.push(CommitReviewGroup {
                        event_type: relationship.event_type,
                        records: [first, second],
                        relationship_id: relationship.id,
                        review_item_ids: [
                            relationship.first_review_item_id,
                            relationship.second_review_item_id,
                        ],
                    });
                }
                PreparedReviewEntry::Outcome(outcome) => outcomes.push(outcome),
            }
        }
        Ok((groups, outcomes))
    }

    fn review_item_record_ids(
        &self,
        review_item_ids: &[String],
    ) -> StoreResult<HashMap<String, String>> {
        if review_item_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = vec!["?"; review_item_ids.len()].join(", ");
        let mut statement = self.connection.prepare(&format!(
            "SELECT id, external_record_id FROM review_items WHERE id IN ({placeholders})",
        ))?;
        let mut record_ids = HashMap::new();
        for row in statement
            .query_map(rusqlite::params_from_iter(review_item_ids.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
        {
            let (review_item_id, record_id) = row?;
            record_ids.entry(review_item_id).or_insert(record_id);
        }
        Ok(record_ids)
    }

    fn relationships_for_review_items(
        &self,
        review_item_ids: &[String],
    ) -> StoreResult<HashMap<String, Vec<StoredReviewRelationship>>> {
        if review_item_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = vec!["?"; review_item_ids.len()].join(", ");
        let mut statement = self.connection.prepare(&format!(
            "SELECT id, event_type, first_external_record_id, second_external_record_id, \
                    first_review_item_id, second_review_item_id, allocation_value, unit, status \
             FROM review_relationships \
             WHERE first_review_item_id IN ({placeholders}) \
                OR second_review_item_id IN ({placeholders}) \
             ORDER BY created_at, id",
        ))?;
        let mut parameters = review_item_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        parameters.extend(review_item_ids.iter().map(String::as_str));
        let mut by_review_item: HashMap<String, Vec<StoredReviewRelationship>> = HashMap::new();
        for row in statement.query_map(rusqlite::params_from_iter(parameters), |row| {
            Ok(StoredReviewRelationship {
                id: row.get(0)?,
                event_type: row.get(1)?,
                first_record_id: row.get(2)?,
                second_record_id: row.get(3)?,
                first_review_item_id: row.get(4)?,
                second_review_item_id: row.get(5)?,
                allocation_value: row.get(6)?,
                unit: row.get(7)?,
                status: row.get(8)?,
            })
        })? {
            let relationship = row?;
            by_review_item
                .entry(relationship.first_review_item_id.clone())
                .or_default()
                .push(relationship.clone());
            by_review_item
                .entry(relationship.second_review_item_id.clone())
                .or_default()
                .push(relationship);
        }
        Ok(by_review_item)
    }

    fn core_records_by_id(
        &self,
        record_ids: &[String],
    ) -> StoreResult<HashMap<String, CoreReviewRecord>> {
        if record_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = vec!["?"; record_ids.len()].join(", ");
        let mut statement = self.connection.prepare(&format!(
            "SELECT external_records.id, external_records.account_id, accounts.account_type, \
                    external_records.currency, external_records.posted_on, \
                    external_records.account_balance_delta, instruments.id \
             FROM external_records \
             JOIN accounts ON accounts.id = external_records.account_id \
             JOIN instruments ON instruments.currency = external_records.currency \
                AND instruments.instrument_type = 'fiat_currency' \
             WHERE external_records.id IN ({placeholders}) \
               AND external_records.status IN ('staged', 'review')",
        ))?;
        let mut records = HashMap::new();
        for row in statement.query_map(
            rusqlite::params_from_iter(record_ids.iter()),
            core_review_record_from_row,
        )? {
            let record = row?;
            records.entry(record.id.clone()).or_insert(record);
        }
        Ok(records)
    }
}
