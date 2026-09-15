use super::tests::{open_store, prepared_repayment, seed_review_repayment};
use super::*;

const COMMITTED_VERSION_FINALITY_TRIGGER: &str = "\
CREATE TRIGGER external_record_committed_version_is_final \
BEFORE INSERT ON external_records \
WHEN EXISTS( \
  SELECT 1 FROM external_records \
  WHERE stable_record_key = NEW.stable_record_key AND status = 'committed' \
) \
BEGIN \
  SELECT RAISE(ABORT, 'committed stable_record_key cannot gain a new version'); \
END;";

fn commit_first_repayment(store: &mut ManualImportStore) {
    store
        .accept_review_relationship(
            "review-record-hsbc-cash",
            1,
            "record-dbs-card",
            1,
            &prepared_repayment(),
        )
        .expect("accept relationship");
    let job = store
        .enqueue_commit_review_batch(&[
            "review-record-hsbc-cash".to_owned(),
            "review-record-dbs-card".to_owned(),
        ])
        .expect("enqueue batch");
    let claimed = store
        .claim_review_batch(&job.job_id, "test-worker")
        .expect("claim batch")
        .expect("claimed job");
    let (groups, _) = store
        .prepare_commit_review_groups(&claimed)
        .expect("prepare groups");
    let outcome = store
        .commit_prepared_review_group(&claimed, &groups[0], &prepared_repayment())
        .expect("commit group");
    assert_eq!(outcome.status, ReviewBatchGroupStatus::Committed);
}

/// A pre-#134 Vault accepted a successor version even though the key already
/// had a committed one. Holding the finality trigger back is how the existing
/// dirty-state tests rebuild that Vault.
fn hold_back_committed_version_finality(store: &ManualImportStore) {
    store
        .connection
        .execute(
            "DROP TRIGGER external_record_committed_version_is_final",
            [],
        )
        .expect("drop finality trigger to simulate a pre-migration dirty Vault");
}

fn stage_reparsed_record(store: &ManualImportStore, id: &str, stable_key: &str, version: i64) {
    let (document_id, parse_run_id, account_id, posted_on) = match id {
        "record-hsbc-cash-v2" => (
            "document-hsbc-june",
            "parse-document-hsbc-june",
            "account-hsbc-cash",
            "2026-06-30",
        ),
        "record-dbs-card-v2" | "record-dbs-other" => (
            "document-dbs-july",
            "parse-document-dbs-july",
            "account-dbs-card",
            "2026-07-01",
        ),
        other => panic!("unknown reparsed record {other}"),
    };
    store
        .connection
        .execute(
            "INSERT INTO external_records( \
               id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
               status, record_type, event_type, posted_on, amount_value, currency, \
               account_balance_delta, raw_json, validation_json \
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'review', 'transaction', \
                       'credit_card_repayment', ?7, '750.00', 'SGD', '-750.00', \
                       '{\"private\":\"must-not-leak\"}', '{\"internal\":true}')",
            params![
                id,
                parse_run_id,
                document_id,
                account_id,
                stable_key,
                version,
                posted_on
            ],
        )
        .expect("stage reparsed record");
}

/// Reproduces the pre-#134 write order that committed a second ledger event for
/// a reparse: the event legs and match edges are written while its records are
/// still uncommitted, and the event and records flip to committed afterwards.
fn commit_second_event(store: &ManualImportStore, event_id: &str, record_ids: &[&str]) {
    store
        .connection
        .execute(
            "INSERT INTO ledger_events( \
               id, event_type, event_class, event_date, status, commit_idempotency_key \
             ) VALUES (?1, 'credit_card_repayment', 'posting', '2026-06-30', 'pending', ?1)",
            [event_id],
        )
        .expect("insert duplicate event");
    for (index, account_id) in ["account-dbs-card", "account-hsbc-cash"].iter().enumerate() {
        store
            .connection
            .execute(
                "INSERT INTO ledger_legs( \
                   id, ledger_event_id, account_id, instrument_id, amount_value, currency \
                 ) VALUES (?1, ?2, ?3, 'instrument-sgd', '-750.00', 'SGD')",
                params![
                    format!("{event_id}:leg:{}", index + 1),
                    event_id,
                    account_id
                ],
            )
            .expect("insert duplicate leg");
    }
    for record_id in record_ids {
        store
            .connection
            .execute(
                "INSERT INTO match_edges( \
                   external_record_id, ledger_event_id, allocation_value, unit, \
                   review_status, unmatched_remainder_value \
                 ) VALUES (?1, ?2, '750.00', 'SGD', 'confirmed', '0')",
                params![record_id, event_id],
            )
            .expect("insert duplicate match edge");
    }
    store
        .connection
        .execute(
            "UPDATE ledger_events SET status = 'committed' WHERE id = ?1",
            [event_id],
        )
        .expect("commit duplicate event");
    for record_id in record_ids {
        store
            .connection
            .execute(
                "UPDATE external_records SET status = 'committed' WHERE id = ?1",
                [record_id],
            )
            .expect("commit duplicate record");
    }
}

fn ledger_event_id_for(store: &ManualImportStore, record_id: &str) -> String {
    store
        .connection
        .query_row(
            "SELECT ledger_event_id FROM match_edges WHERE external_record_id = ?1",
            [record_id],
            |row| row.get(0),
        )
        .expect("read committed ledger event")
}

#[test]
fn reports_no_duplicate_commit_for_a_vault_with_one_committed_version_per_key() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    commit_first_repayment(&mut store);

    let rows = store
        .audit_duplicate_committed_versions()
        .expect("audit a clean Vault");

    assert!(rows.is_empty(), "clean Vault reported {rows:?}");
}

#[test]
fn lists_every_committed_version_of_a_double_committed_key_with_its_reversal_precondition() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    commit_first_repayment(&mut store);
    let first_event_id = ledger_event_id_for(&store, "record-hsbc-cash");

    hold_back_committed_version_finality(&store);
    stage_reparsed_record(&store, "record-hsbc-cash-v2", "hsbc-card-payment", 2);
    stage_reparsed_record(&store, "record-dbs-card-v2", "dbs-card-payment", 2);
    commit_second_event(
        &store,
        "event-duplicate",
        &["record-hsbc-cash-v2", "record-dbs-card-v2"],
    );
    store
        .connection
        .execute(COMMITTED_VERSION_FINALITY_TRIGGER, [])
        .expect("re-create finality trigger");

    let rows = store
        .audit_duplicate_committed_versions()
        .expect("audit a double committed Vault");

    let listing = rows
        .iter()
        .map(|row| {
            (
                row.stable_record_key.as_str(),
                row.external_record_id.as_str(),
                row.version,
                row.version_rank,
                row.ledger_event_id.as_deref(),
                row.reversal_safe,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        listing,
        vec![
            (
                "dbs-card-payment",
                "record-dbs-card",
                1,
                1,
                Some(first_event_id.as_str()),
                false,
            ),
            (
                "dbs-card-payment",
                "record-dbs-card-v2",
                2,
                2,
                Some("event-duplicate"),
                true,
            ),
            (
                "hsbc-card-payment",
                "record-hsbc-cash",
                1,
                1,
                Some(first_event_id.as_str()),
                false,
            ),
            (
                "hsbc-card-payment",
                "record-hsbc-cash-v2",
                2,
                2,
                Some("event-duplicate"),
                true,
            ),
        ]
    );
    for row in &rows {
        assert!(!row.ledger_event_is_reversal);
        assert!(!row.ledger_event_has_reversal);
        assert_eq!(row.match_review_status.as_deref(), Some("confirmed"));
        assert_eq!(row.match_unit.as_deref(), Some("SGD"));
        assert_eq!(row.allocation_value.as_deref(), Some("750.00"));
        assert_eq!(
            row.ledger_event_type.as_deref(),
            Some("credit_card_repayment")
        );
    }
}

#[test]
fn reports_a_duplicate_event_that_also_carries_a_canonical_edge_as_unsafe_to_reverse() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    commit_first_repayment(&mut store);
    let first_event_id = ledger_event_id_for(&store, "record-hsbc-cash");

    hold_back_committed_version_finality(&store);
    stage_reparsed_record(&store, "record-hsbc-cash-v2", "hsbc-card-payment", 2);
    stage_reparsed_record(&store, "record-dbs-other", "dbs-card-payment-2", 1);
    commit_second_event(
        &store,
        "event-duplicate",
        &["record-hsbc-cash-v2", "record-dbs-other"],
    );
    store
        .connection
        .execute(COMMITTED_VERSION_FINALITY_TRIGGER, [])
        .expect("re-create finality trigger");

    let rows = store
        .audit_duplicate_committed_versions()
        .expect("audit a partially duplicated Vault");

    let listing = rows
        .iter()
        .map(|row| {
            (
                row.stable_record_key.as_str(),
                row.external_record_id.as_str(),
                row.version,
                row.version_rank,
                row.ledger_event_id.as_deref(),
                row.reversal_safe,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        listing,
        vec![
            (
                "hsbc-card-payment",
                "record-hsbc-cash",
                1,
                1,
                Some(first_event_id.as_str()),
                false,
            ),
            (
                "hsbc-card-payment",
                "record-hsbc-cash-v2",
                2,
                2,
                Some("event-duplicate"),
                false,
            ),
        ]
    );
}

const COMMITTED_LEDGER_EVENT_IMMUTABILITY_TRIGGER: &str = "\
CREATE TRIGGER committed_ledger_event_is_immutable \
BEFORE UPDATE ON ledger_events \
WHEN OLD.status = 'committed' \
BEGIN \
  SELECT RAISE(ABORT, 'committed ledger event is immutable'); \
END;";

fn duplicate_rows(rows: &[DuplicateCommittedVersionAuditRow]) -> Vec<(&str, Option<&str>, bool)> {
    rows.iter()
        .filter(|row| row.version_rank > 1)
        .map(|row| {
            (
                row.external_record_id.as_str(),
                row.ledger_event_status.as_deref(),
                row.reversal_safe,
            )
        })
        .collect()
}

/// `reversal_safe` promises the whole precondition, so it must also refuse an
/// event that never committed and an event that is itself a reversal. Both
/// anomalies are built directly here; the flag may not depend on a consumer
/// re-reading event status before inverting.
#[test]
fn refuses_to_report_an_uncommitted_or_already_reversed_duplicate_event_as_reversible() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    commit_first_repayment(&mut store);
    let first_event_id = ledger_event_id_for(&store, "record-hsbc-cash");

    hold_back_committed_version_finality(&store);
    stage_reparsed_record(&store, "record-hsbc-cash-v2", "hsbc-card-payment", 2);
    stage_reparsed_record(&store, "record-dbs-card-v2", "dbs-card-payment", 2);
    commit_second_event(
        &store,
        "event-duplicate",
        &["record-hsbc-cash-v2", "record-dbs-card-v2"],
    );
    store
        .connection
        .execute(COMMITTED_VERSION_FINALITY_TRIGGER, [])
        .expect("re-create finality trigger");

    store
        .connection
        .execute("DROP TRIGGER committed_ledger_event_is_immutable", [])
        .expect("drop event immutability trigger to build the anomaly");
    store
        .connection
        .execute(
            "UPDATE ledger_events SET status = 'pending' WHERE id = 'event-duplicate'",
            [],
        )
        .expect("leave the duplicate event uncommitted");

    let pending_event_rows = store
        .audit_duplicate_committed_versions()
        .expect("audit a duplicate behind a pending event");
    assert_eq!(
        duplicate_rows(&pending_event_rows),
        vec![
            ("record-dbs-card-v2", Some("pending"), false),
            ("record-hsbc-cash-v2", Some("pending"), false),
        ]
    );

    store
        .connection
        .execute(
            "UPDATE ledger_events SET status = 'committed', reverses_event_id = ?1 \
             WHERE id = 'event-duplicate'",
            [&first_event_id],
        )
        .expect("turn the duplicate event into a reversal");
    store
        .connection
        .execute(COMMITTED_LEDGER_EVENT_IMMUTABILITY_TRIGGER, [])
        .expect("re-create event immutability trigger");

    let reversal_event_rows = store
        .audit_duplicate_committed_versions()
        .expect("audit a duplicate that is itself a reversal");
    assert_eq!(
        duplicate_rows(&reversal_event_rows),
        vec![
            ("record-dbs-card-v2", Some("committed"), false),
            ("record-hsbc-cash-v2", Some("committed"), false),
        ]
    );
    assert!(
        reversal_event_rows
            .iter()
            .filter(|row| row.version_rank == 1)
            .all(|row| row.ledger_event_has_reversal && !row.reversal_safe)
    );
}
