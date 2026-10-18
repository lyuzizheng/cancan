//! Review-relationship lifecycle transitions.
//!
//! A pair of staged records can be accepted, and its row moves with the records
//! it names: a reparse supersedes the live version of one side, an account
//! dismissal removes it, and re-pairing the same two records revives the pair.
//! Every transition runs inside the caller's transaction, so a retirement and
//! its invalidation commit or roll back together, and every statement moves only
//! `accepted` rows — committed history and pairs on other records keep their
//! state exactly as they were.

use super::*;

/// Invalidates the accepted pairs naming a record version that a reparse just
/// superseded under `stable_record_key`. The caller has already retired those
/// versions in the same transaction.
pub(super) fn invalidate_pairs_retired_by_supersede(
    transaction: &Transaction<'_>,
    stable_record_key: &str,
) -> StoreResult<()> {
    transaction.execute(
        "UPDATE review_relationships SET status = 'invalidated' \
         WHERE status = 'accepted' AND ( \
           first_external_record_id IN ( \
             SELECT id FROM external_records \
             WHERE stable_record_key = ?1 AND status = 'superseded' \
           ) OR second_external_record_id IN ( \
             SELECT id FROM external_records \
             WHERE stable_record_key = ?1 AND status = 'superseded' \
           ) \
         )",
        [stable_record_key],
    )?;
    Ok(())
}

/// Invalidates the accepted pairs naming a record that an account dismissal
/// removed. The caller has already removed those records in the same
/// transaction.
pub(super) fn invalidate_pairs_retired_by_account_removal(
    transaction: &Transaction<'_>,
    account_id: &str,
) -> StoreResult<()> {
    transaction.execute(
        "UPDATE review_relationships SET status = 'invalidated' \
         WHERE status = 'accepted' AND ( \
           first_external_record_id IN ( \
             SELECT id FROM external_records WHERE account_id = ?1 AND status = 'removed' \
           ) OR second_external_record_id IN ( \
             SELECT id FROM external_records WHERE account_id = ?1 AND status = 'removed' \
           ) \
         )",
        [account_id],
    )?;
    Ok(())
}

/// Settles the pair that already has a row for these two records, when it has
/// one, and reports whether it did.
///
/// A reparse or a dismissal retires an accepted pair; once both sides are live
/// again the user re-pairs the same records, so the row is revived in place —
/// the record pair keeps one identity — and re-pointed at the review items that
/// are open now, so a pending commit finds the pair again. A pair that is still
/// `accepted` or already `committed` is in force and stays exactly as it is.
pub(super) fn settle_existing_pair(
    transaction: &Transaction<'_>,
    first_record_id: &str,
    second_record_id: &str,
    first_review_item_id: &str,
    second_review_item_id: &str,
    event_type: &str,
) -> StoreResult<bool> {
    let existing = transaction
        .query_row(
            "SELECT id, status FROM review_relationships \
             WHERE first_external_record_id = ?1 AND second_external_record_id = ?2",
            params![first_record_id, second_record_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((relationship_id, status)) = existing else {
        return Ok(false);
    };
    if status == "invalidated" {
        transaction.execute(
            "UPDATE review_relationships \
             SET status = 'accepted', first_review_item_id = ?2, second_review_item_id = ?3 \
             WHERE id = ?1 AND status = 'invalidated'",
            params![relationship_id, first_review_item_id, second_review_item_id],
        )?;
        record_acceptance(transaction, first_record_id, second_record_id, event_type)?;
    }
    Ok(true)
}

/// Records one user acceptance of a record pair in the audit trail. A first
/// acceptance and the re-pairing of a previously invalidated pair write the same
/// event, because the user confirmed the same contract again.
pub(super) fn record_acceptance(
    transaction: &Transaction<'_>,
    first_record_id: &str,
    second_record_id: &str,
    event_type: &str,
) -> StoreResult<()> {
    transaction.execute(
        "INSERT INTO audit_log( \
           id, entity_type, entity_id, action, actor, reason, source_ref, policy_version \
         ) VALUES (?1, 'review_relationship', ?2, 'review_relationship_accepted', \
                   'user', 'confirmed_relationship', ?3, ?4)",
        params![
            new_audit_id(),
            format!("{first_record_id}:{second_record_id}"),
            event_type,
            REVIEW_POLICY_VERSION,
        ],
    )?;
    Ok(())
}
