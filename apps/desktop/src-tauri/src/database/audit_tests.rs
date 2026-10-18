use super::tests::{open_store, prepared_repayment, seed_review_repayment};
use super::*;

const COMMITTED_VERSION_FINALITY_INSERT_TRIGGER: &str = "\
CREATE TRIGGER external_record_committed_version_is_final \
BEFORE INSERT ON external_records \
WHEN EXISTS( \
  SELECT 1 FROM external_records \
  WHERE stable_record_key = NEW.stable_record_key AND status = 'committed' \
) \
BEGIN \
  SELECT RAISE(ABORT, 'committed stable_record_key cannot gain a new version'); \
END;";

const COMMITTED_VERSION_FINALITY_UPDATE_TRIGGER: &str = "\
CREATE TRIGGER external_record_committed_update_is_final \
BEFORE UPDATE OF status ON external_records \
WHEN NEW.status = 'committed' \
  AND OLD.status <> 'committed' \
  AND EXISTS( \
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
/// had a committed one. Holding both finality triggers back is how the existing
/// dirty-state tests rebuild that Vault: the insert guard refuses the successor
/// row and the update guard refuses the `status = 'committed'` write that makes
/// it a duplicate.
fn hold_back_committed_version_finality(store: &ManualImportStore) {
    store
        .connection
        .execute_batch(
            "DROP TRIGGER external_record_committed_version_is_final; \
             DROP TRIGGER external_record_committed_update_is_final;",
        )
        .expect("drop finality triggers to simulate a pre-migration dirty Vault");
}

fn restore_committed_version_finality(store: &ManualImportStore) {
    store
        .connection
        .execute(COMMITTED_VERSION_FINALITY_INSERT_TRIGGER, [])
        .expect("re-create insert finality trigger");
    store
        .connection
        .execute(COMMITTED_VERSION_FINALITY_UPDATE_TRIGGER, [])
        .expect("re-create update finality trigger");
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
    restore_committed_version_finality(&store);

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
    restore_committed_version_finality(&store);

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

fn duplicate_rows(
    rows: &[DuplicateCommittedVersionAuditRow],
) -> Vec<(&str, Option<&str>, bool, bool)> {
    rows.iter()
        .filter(|row| row.version_rank > 1)
        .map(|row| {
            (
                row.external_record_id.as_str(),
                row.ledger_event_status.as_deref(),
                row.ledger_event_has_reversal,
                row.reversal_safe,
            )
        })
        .collect()
}

/// `reversal_safe` promises the whole precondition, so it must also refuse an
/// event that never committed, an event that is itself a reversal, and an
/// event that already carries one. The three anomalies are built directly
/// here; the flag may not depend on a consumer re-reading event status or the
/// reversal column before inverting.
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
    restore_committed_version_finality(&store);

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
            ("record-dbs-card-v2", Some("pending"), false, false),
            ("record-hsbc-cash-v2", Some("pending"), false, false),
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

    let reversal_event_rows = store
        .audit_duplicate_committed_versions()
        .expect("audit a duplicate that is itself a reversal");
    assert_eq!(
        duplicate_rows(&reversal_event_rows),
        vec![
            ("record-dbs-card-v2", Some("committed"), false, false),
            ("record-hsbc-cash-v2", Some("committed"), false, false),
        ]
    );
    assert!(
        reversal_event_rows
            .iter()
            .filter(|row| row.version_rank == 1)
            .all(|row| row.ledger_event_has_reversal && !row.reversal_safe)
    );

    store
        .connection
        .execute(
            "UPDATE ledger_events SET reverses_event_id = NULL WHERE id = 'event-duplicate'",
            [],
        )
        .expect("restore the duplicate event to a plain posting");
    store
        .connection
        .execute(
            "INSERT INTO ledger_events( \
               id, event_type, event_class, event_date, status, commit_idempotency_key, \
               reverses_event_id \
             ) VALUES ('event-reversal', 'credit_card_repayment_reversal', 'posting', \
                       '2026-06-30', 'committed', 'repair:event-duplicate', 'event-duplicate')",
            [],
        )
        .expect("append the reversal a repair would write");
    store
        .connection
        .execute(COMMITTED_LEDGER_EVENT_IMMUTABILITY_TRIGGER, [])
        .expect("re-create event immutability trigger");

    let repaired_rows = store
        .audit_duplicate_committed_versions()
        .expect("audit a duplicate event that a repair already reversed");
    assert_eq!(
        duplicate_rows(&repaired_rows),
        vec![
            ("record-dbs-card-v2", Some("committed"), true, false),
            ("record-hsbc-cash-v2", Some("committed"), true, false),
        ]
    );
}

fn finality_statuses(store: &ManualImportStore, stable_key: &str) -> Vec<(String, String)> {
    let mut statement = store
        .connection
        .prepare(
            "SELECT id, status FROM external_records \
             WHERE stable_record_key = ?1 ORDER BY version",
        )
        .expect("prepare status read");
    statement
        .query_map([stable_key], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("read statuses")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect statuses")
}

fn assert_finality_rejection(result: rusqlite::Result<usize>, action: &str) {
    let error = result.expect_err(action);
    let rendered = error.to_string();
    assert!(
        rendered.contains("committed stable_record_key cannot gain a new version"),
        "unexpected error: {rendered}"
    );
}

/// Migration 0013 guarded only `INSERT`, while both commit paths flip the
/// record to `committed` with `UPDATE`. The direct write that used to create a
/// second committed version of one key must now fail at the schema boundary.
#[test]
fn schema_migration_16_blocks_a_direct_update_to_a_second_committed_version() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    stage_reparsed_record(&store, "record-hsbc-cash-v2", "hsbc-card-payment", 2);
    store
        .connection
        .execute(
            "UPDATE external_records SET status = 'committed' WHERE id = 'record-hsbc-cash'",
            [],
        )
        .expect("commit the first version");

    assert_finality_rejection(
        store.connection.execute(
            "UPDATE external_records SET status = 'committed' WHERE id = 'record-hsbc-cash-v2'",
            [],
        ),
        "a second committed version must be rejected",
    );

    assert_eq!(
        finality_statuses(&store, "hsbc-card-payment"),
        vec![
            ("record-hsbc-cash".to_owned(), "committed".to_owned()),
            ("record-hsbc-cash-v2".to_owned(), "review".to_owned()),
        ]
    );
    let (version, amount, currency): (i64, Option<String>, Option<String>) = store
        .connection
        .query_row(
            "SELECT version, amount_value, currency FROM external_records \
             WHERE id = 'record-hsbc-cash-v2'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read rejected row");
    assert_eq!(version, 2);
    assert_eq!(amount.as_deref(), Some("750.00"));
    assert_eq!(currency.as_deref(), Some("SGD"));
}

/// One statement that commits both versions of a key is the same duplicate
/// through a different shape; the rejected write must leave both rows in their
/// pre-update state, so the statement rolls back as a whole.
#[test]
fn schema_migration_16_rejects_one_statement_that_commits_two_versions() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    stage_reparsed_record(&store, "record-hsbc-cash-v2", "hsbc-card-payment", 2);

    assert_finality_rejection(
        store.connection.execute(
            "UPDATE external_records SET status = 'committed' \
             WHERE id IN ('record-hsbc-cash', 'record-hsbc-cash-v2')",
            [],
        ),
        "committing two versions in one statement must be rejected",
    );

    assert_eq!(
        finality_statuses(&store, "hsbc-card-payment"),
        vec![
            ("record-hsbc-cash".to_owned(), "review".to_owned()),
            ("record-hsbc-cash-v2".to_owned(), "review".to_owned()),
        ]
    );
}

/// The guard is specific to gaining committed status: a first commit and every
/// non-committed transition on a key with a committed sibling stay legal.
#[test]
fn schema_migration_16_permits_a_first_commit_and_uncommitted_transitions() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    stage_reparsed_record(&store, "record-hsbc-cash-v2", "hsbc-card-payment", 2);

    store
        .connection
        .execute(
            "UPDATE external_records SET status = 'committed' WHERE id = 'record-dbs-card'",
            [],
        )
        .expect("commit a first version");
    store
        .connection
        .execute(
            "UPDATE external_records SET status = 'committed' WHERE id = 'record-hsbc-cash'",
            [],
        )
        .expect("commit another key's first version");
    store
        .connection
        .execute(
            "UPDATE external_records SET status = 'superseded' WHERE id = 'record-hsbc-cash-v2'",
            [],
        )
        .expect("supersede an uncommitted sibling");

    assert_eq!(
        finality_statuses(&store, "hsbc-card-payment"),
        vec![
            ("record-hsbc-cash".to_owned(), "committed".to_owned()),
            ("record-hsbc-cash-v2".to_owned(), "superseded".to_owned()),
        ]
    );
}

/// A Vault written before this guard can already hold two committed versions
/// of one key. The forward migration must apply over that populated state and
/// leave both rows untouched; it cannot repair history.
#[test]
fn schema_migration_16_upgrades_a_populated_vault_without_rewriting_legacy_duplicates() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let database_path = root.path().join(DATABASE_FILE_NAME);
    let mut connection = open_encrypted_database(
        &database_path,
        &super::tests::KEY,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .expect("open encrypted database");
    let fix_position = MIGRATIONS
        .iter()
        .position(|migration| migration.version == 16)
        .expect("migration 16 is registered");
    apply_migration_set(&mut connection, &MIGRATIONS[..fix_position])
        .expect("apply the pre-fix schema");

    connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES ('source-legacy', 'dbs', 'DBS', 'bank')",
            [],
        )
        .expect("seed money source");
    connection
        .execute(
            "INSERT INTO source_documents( \
               id, money_source_id, file_sha256, semantic_document_key, \
               original_filename, mime_type, byte_size, file_state \
             ) VALUES ('document-legacy', 'source-legacy', ?1, 'legacy:doc', \
                       'legacy.csv', 'text/csv', 1, 'missing')",
            ["a".repeat(64)],
        )
        .expect("seed source document");
    connection
        .execute(
            "INSERT INTO parse_runs( \
               id, source_document_id, normalization_profile_id, logical_run_key, \
               profile_json, input_hash, status \
             ) VALUES ('parse-legacy', 'document-legacy', 'synthetic-review-v1', \
                       'legacy-run', '{}', '', 'validated')",
            [],
        )
        .expect("seed parse run");
    for version in 1..=2 {
        connection
            .execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, stable_record_key, version, \
                   status, record_type, amount_value, currency, raw_json, validation_json \
                 ) VALUES (?1, 'parse-legacy', 'document-legacy', 'legacy-key', ?2, \
                           'review', 'transaction', '100.00', 'SGD', '{}', '{}')",
                params![format!("record-legacy-v{version}"), version],
            )
            .expect("seed legacy version");
    }
    for version in 1..=2 {
        connection
            .execute(
                "UPDATE external_records SET status = 'committed' WHERE id = ?1",
                [format!("record-legacy-v{version}")],
            )
            .expect("pre-fix commit of a legacy version");
    }

    apply_migration_set(&mut connection, MIGRATIONS).expect("apply the forward migration");

    let applied: i64 = connection
        .query_row(
            "SELECT count(*) FROM schema_migrations WHERE version = 16",
            [],
            |row| row.get(0),
        )
        .expect("read applied migration");
    assert_eq!(applied, 1);
    let legacy_rows: Vec<(String, i64, String)> = {
        let mut statement = connection
            .prepare(
                "SELECT id, version, status FROM external_records \
                 WHERE stable_record_key = 'legacy-key' ORDER BY version",
            )
            .expect("prepare legacy read");
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("read legacy rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect legacy rows")
    };
    assert_eq!(
        legacy_rows,
        vec![
            ("record-legacy-v1".to_owned(), 1, "committed".to_owned()),
            ("record-legacy-v2".to_owned(), 2, "committed".to_owned()),
        ]
    );
    let guard_installed: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master \
             WHERE type = 'trigger' AND name = 'external_record_committed_update_is_final'",
            [],
            |row| row.get(0),
        )
        .expect("read trigger registry");
    assert_eq!(guard_installed, 1);
}
