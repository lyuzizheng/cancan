use super::*;

// Read-only audit for the one dirty state migration 0013 deliberately leaves
// alone: two committed versions of one `stable_record_key`, each with its own
// committed `ledger_event` and `match_edges`. Reparse-before-commit-finality
// could produce that, and the ledger then counts the same money movement
// twice. Migration 0013 only supersedes non-committed descendants, because
// reversing an already-posted event needs explicit human confirmation.
//
// The query reads four tables and writes nothing. `version_rank` orders the
// committed versions of a key by `(version, created_at, id)`, so rank 1 is the
// version a repair keeps. `reversal_safe` carries the whole precondition for
// appending a full inversion of that row's ledger event, so a consumer never
// has to re-filter a flag named for safety:
//
//   - the event exists, is itself committed, and is not itself a reversal;
//   - the event does not already have a reversal, so a second repair pass
//     cannot invert it again;
//   - every match edge of the event points at a non-canonical committed
//     version, so inverting it cannot erase a canonical record's effect.
//
// The middle two conditions are not about duplicates at all: inverting a
// pending event, re-inverting a reversal, or reversing an event twice all
// write a wrong ledger, and a duplicate record can carry any of them. Because
// the flag carries every clause, `reversal_safe = false` with
// `ledger_event_has_reversal = true` reads as already repaired, while
// `false` with `false` reads as needing a human decision.
//
// The last condition is why an event can be unsafe even when it heads a
// duplicate: a commit writes its edges while both records are still
// uncommitted, so the event may also carry an edge to another key's only
// committed version.
pub(crate) const DUPLICATE_COMMITTED_VERSIONS_SQL: &str = "\
WITH committed_versions AS ( \
  SELECT external_records.id AS external_record_id, \
         external_records.stable_record_key AS stable_record_key, \
         ROW_NUMBER() OVER ( \
           PARTITION BY external_records.stable_record_key \
           ORDER BY external_records.version, external_records.created_at, external_records.id \
         ) AS version_rank \
  FROM external_records \
  WHERE external_records.status = 'committed' \
) \
SELECT committed_versions.stable_record_key, \
       committed_versions.external_record_id, \
       external_records.version, \
       committed_versions.version_rank, \
       external_records.source_document_id, \
       external_records.event_type, \
       external_records.posted_on, \
       external_records.amount_value, \
       external_records.currency, \
       ledger_events.id, \
       ledger_events.event_type, \
       ledger_events.event_date, \
       ledger_events.status, \
       ledger_events.reverses_event_id IS NOT NULL, \
       EXISTS(SELECT 1 FROM ledger_events reversal \
              WHERE reversal.reverses_event_id = ledger_events.id), \
       match_edges.allocation_value, \
       match_edges.unit, \
       match_edges.review_status, \
       ledger_events.id IS NOT NULL \
         AND ledger_events.status = 'committed' \
         AND ledger_events.reverses_event_id IS NULL \
         AND NOT EXISTS( \
           SELECT 1 FROM ledger_events reversal \
           WHERE reversal.reverses_event_id = ledger_events.id \
         ) \
         AND NOT EXISTS( \
           SELECT 1 FROM match_edges peer_edge \
           LEFT JOIN committed_versions peer \
             ON peer.external_record_id = peer_edge.external_record_id \
            AND peer.version_rank > 1 \
           WHERE peer_edge.ledger_event_id = ledger_events.id \
             AND peer.external_record_id IS NULL \
         ) \
FROM committed_versions \
JOIN external_records ON external_records.id = committed_versions.external_record_id \
LEFT JOIN match_edges ON match_edges.external_record_id = committed_versions.external_record_id \
LEFT JOIN ledger_events ON ledger_events.id = match_edges.ledger_event_id \
WHERE ( \
  SELECT COUNT(*) FROM committed_versions sibling \
  WHERE sibling.stable_record_key = committed_versions.stable_record_key \
) > 1 \
ORDER BY committed_versions.stable_record_key, external_records.version, ledger_events.id";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub(crate) struct DuplicateCommittedVersionAuditRow {
    pub(crate) stable_record_key: String,
    pub(crate) external_record_id: String,
    pub(crate) source_document_id: String,
    pub(crate) version: i64,
    pub(crate) version_rank: i64,
    pub(crate) record_event_type: Option<String>,
    pub(crate) posted_on: Option<String>,
    pub(crate) amount_value: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) ledger_event_id: Option<String>,
    pub(crate) ledger_event_type: Option<String>,
    pub(crate) ledger_event_date: Option<String>,
    pub(crate) ledger_event_status: Option<String>,
    pub(crate) ledger_event_is_reversal: bool,
    pub(crate) ledger_event_has_reversal: bool,
    pub(crate) allocation_value: Option<String>,
    pub(crate) match_unit: Option<String>,
    pub(crate) match_review_status: Option<String>,
    pub(crate) reversal_safe: bool,
}

impl ManualImportStore {
    pub(crate) fn audit_duplicate_committed_versions(
        &self,
    ) -> StoreResult<Vec<DuplicateCommittedVersionAuditRow>> {
        let mut statement = self.connection.prepare(DUPLICATE_COMMITTED_VERSIONS_SQL)?;
        let rows = statement.query_map([], |row| {
            Ok(DuplicateCommittedVersionAuditRow {
                stable_record_key: row.get(0)?,
                external_record_id: row.get(1)?,
                version: row.get(2)?,
                version_rank: row.get(3)?,
                source_document_id: row.get(4)?,
                record_event_type: row.get(5)?,
                posted_on: row.get(6)?,
                amount_value: row.get(7)?,
                currency: row.get(8)?,
                ledger_event_id: row.get(9)?,
                ledger_event_type: row.get(10)?,
                ledger_event_date: row.get(11)?,
                ledger_event_status: row.get(12)?,
                ledger_event_is_reversal: row.get(13)?,
                ledger_event_has_reversal: row.get(14)?,
                allocation_value: row.get(15)?,
                match_unit: row.get(16)?,
                match_review_status: row.get(17)?,
                reversal_safe: row.get(18)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
