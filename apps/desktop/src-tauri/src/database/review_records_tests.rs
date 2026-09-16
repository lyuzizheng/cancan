use super::tests::{open_store, prepared_repayment, seed_review_repayment};
use super::*;

/// Commits the seeded HSBC repayment and reparses its document with divergent
/// canonical fields, so one `reparse_divergence` item is attached to the
/// committed record — the only state that produces committed-record review work.
fn committed_record_with_divergence(store: &mut ManualImportStore) -> (String, String) {
    seed_review_repayment(store, false);
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
    let committed = store
        .commit_prepared_review_group(&claimed, &groups[0], &prepared_repayment())
        .expect("commit group");
    assert_eq!(committed.status, ReviewBatchGroupStatus::Committed);

    store
        .connection
        .execute(
            "INSERT INTO jobs(id, job_type, status, related_source_document_id, input_json, lease_owner, lease_until) \
             VALUES ('job-reparse-ack-1', 'parse_document', 'running', 'document-hsbc-june', \
                     '{\"documentId\":\"document-hsbc-june\",\"logicalRunKey\":\"run-ack-1\"}', 'test-worker', datetime('now', '+5 minutes'))",
            [],
        )
        .expect("seed parse job");
    let claim = ParseDocumentClaim {
        claim_token: "test-worker".to_string(),
        document_id: "document-hsbc-june".to_string(),
        job_id: "job-reparse-ack-1".to_string(),
        logical_run_key: "run-ack-1".to_string(),
    };
    let parse_input = ValidatedStructuredParseInput {
        normalization_profile_id: "profile-reparse".to_string(),
        profile_json: "{}".to_string(),
        records: vec![ValidatedExternalRecordInput {
            account_id: "account-hsbc-cash".to_string(),
            account_balance_delta: Some("-800.00".to_string()),
            amount_value: Some("800.00".to_string()),
            currency: Some("SGD".to_string()),
            event_type: Some("credit_card_repayment".to_string()),
            posted_on: Some("2026-06-30".to_string()),
            posting_status: Some("posted".to_string()),
            raw_json: r#"{"private":"changed-data"}"#.to_string(),
            record_type: "transaction".to_string(),
            stable_record_key: "hsbc-card-payment".to_string(),
            validation_json:
                r#"{"schemaValid":true,"rawGrounded":true,"deterministicValidationPassed":true}"#
                    .to_string(),
        }],
    };
    store
        .persist_validated_structured_parse_for_claimed_job(
            "document-hsbc-june",
            &parse_input,
            &claim,
            "input-hash-ack-1",
            "output-hash-ack-1",
        )
        .expect("persist divergent parse");

    let review_item_id: String = store
        .connection
        .query_row(
            "SELECT id FROM review_items \
             WHERE external_record_id = 'record-hsbc-cash' AND reason_code = 'reparse_divergence' \
               AND status = 'open'",
            [],
            |row| row.get(0),
        )
        .expect("one open divergence item");
    ("record-hsbc-cash".to_owned(), review_item_id)
}

fn committed_record_snapshot(
    store: &ManualImportStore,
    record_id: &str,
) -> (i64, String, Option<String>, Option<String>, String) {
    store
        .connection
        .query_row(
            "SELECT version, status, amount_value, account_balance_delta, raw_json \
             FROM external_records WHERE id = ?1",
            [record_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("committed record row")
}

fn table_count(store: &ManualImportStore, table: &str) -> i64 {
    store
        .connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("row count")
}

#[test]
fn committed_record_divergence_item_is_only_acknowledgeable() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let (record_id, review_item_id) = committed_record_with_divergence(&mut store);

    let records_before = table_count(&store, "external_records");
    let events_before = table_count(&store, "ledger_events");
    let edges_before = table_count(&store, "match_edges");
    let committed_before = committed_record_snapshot(&store, &record_id);
    assert_eq!(committed_before.1, "committed");

    // The renderer sees the item and that its record is already committed.
    let listed = store.list_review_items().expect("review list");
    let listed_item = listed
        .iter()
        .find(|item| item.review_item_id == review_item_id)
        .expect("divergence item is listed");
    assert!(listed_item.record_committed);
    assert_eq!(listed_item.reason_code, "reparse_divergence");
    assert_eq!(listed_item.record_id, record_id);
    let detail = store
        .review_item_detail(&review_item_id)
        .expect("review detail read")
        .expect("divergence item has a detail");
    assert!(detail.record_committed);
    assert_eq!(detail.record_version, committed_before.0);

    // A committed record keeps edit and remove closed.
    assert_eq!(
        store
            .edit_review_record(&review_item_id, 1, Some("2026-06-29"), None, None)
            .expect("edit is refused safely"),
        review_conflict("stale_review_item")
    );
    assert_eq!(
        store
            .remove_review_record(&review_item_id, 1)
            .expect("remove is refused safely"),
        review_conflict("stale_review_item")
    );

    let outcome = store
        .acknowledge_review_item(&review_item_id, 1)
        .expect("acknowledge the divergence");
    assert_eq!(outcome.status, ReviewMutationStatus::Acknowledged);
    assert_eq!(outcome.reason, None);

    let item_status: String = store
        .connection
        .query_row(
            "SELECT status FROM review_items WHERE id = ?1",
            [&review_item_id],
            |row| row.get(0),
        )
        .expect("item status");
    assert_eq!(item_status, "dismissed");

    // The committed record, its ledger effects, and every other record are untouched.
    assert_eq!(
        committed_record_snapshot(&store, &record_id),
        committed_before
    );
    assert_eq!(table_count(&store, "external_records"), records_before);
    assert_eq!(table_count(&store, "ledger_events"), events_before);
    assert_eq!(table_count(&store, "match_edges"), edges_before);

    let audit: (String, String, String, String, String, String) = store
        .connection
        .query_row(
            "SELECT entity_type, entity_id, action, actor, source_ref, policy_version \
             FROM audit_log WHERE action = 'review_item_acknowledged'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("acknowledge audit entry");
    assert_eq!(
        audit,
        (
            "external_record".to_owned(),
            record_id.clone(),
            "review_item_acknowledged".to_owned(),
            "user".to_owned(),
            format!("{review_item_id}:1"),
            REVIEW_POLICY_VERSION.to_owned(),
        )
    );

    // The acknowledged item leaves the queue and cannot be acknowledged twice.
    assert!(
        store
            .list_review_items()
            .expect("review list")
            .iter()
            .all(|item| item.review_item_id != review_item_id)
    );
    assert!(
        store
            .review_item_detail(&review_item_id)
            .expect("review detail read")
            .is_none()
    );
    assert_eq!(
        store
            .acknowledge_review_item(&review_item_id, 1)
            .expect("second acknowledge is refused safely"),
        review_conflict("stale_review_item")
    );
}

#[test]
fn acknowledge_refuses_an_uncommitted_review_item() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);

    let listed = store.list_review_items().expect("review list");
    let staged_item = listed
        .iter()
        .find(|item| item.review_item_id == "review-record-hsbc-cash")
        .expect("staged item is listed");
    assert!(!staged_item.record_committed);

    assert_eq!(
        store
            .acknowledge_review_item("review-record-hsbc-cash", 1)
            .expect("uncommitted acknowledge is refused safely"),
        review_conflict("stale_review_item")
    );
    let item_status: String = store
        .connection
        .query_row(
            "SELECT status FROM review_items WHERE id = 'review-record-hsbc-cash'",
            [],
            |row| row.get(0),
        )
        .expect("item status");
    assert_eq!(item_status, "open");
    let acknowledge_audits: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM audit_log WHERE action = 'review_item_acknowledged'",
            [],
            |row| row.get(0),
        )
        .expect("acknowledge audits");
    assert_eq!(acknowledge_audits, 0);
}
