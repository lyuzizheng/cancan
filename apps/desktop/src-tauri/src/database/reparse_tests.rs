use super::tests::{KEY, open_store, prepared_repayment, seed_review_repayment};
use super::*;

#[test]
fn committed_record_reparse_with_identical_output_skips_insert_and_emits_audit_log() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
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

    let status: String = store
        .connection
        .query_row(
            "SELECT status FROM external_records WHERE id = 'record-hsbc-cash'",
            [],
            |row| row.get(0),
        )
        .expect("status");
    assert_eq!(status, "committed");

    store
        .connection
        .execute(
            "INSERT INTO jobs(id, job_type, status, related_source_document_id, input_json, lease_owner, lease_until) \
             VALUES ('job-reparse-1', 'parse_document', 'running', 'document-hsbc-june', \
                     '{\"documentId\":\"document-hsbc-june\",\"logicalRunKey\":\"run-reparse-1\"}', 'test-worker', datetime('now', '+5 minutes'))",
            [],
        )
        .expect("seed parse job");
    let claim = ParseDocumentClaim {
        claim_token: "test-worker".to_string(),
        document_id: "document-hsbc-june".to_string(),
        job_id: "job-reparse-1".to_string(),
        logical_run_key: "run-reparse-1".to_string(),
    };
    let parse_input = ValidatedStructuredParseInput {
        normalization_profile_id: "profile-reparse".to_string(),
        profile_json: "{}".to_string(),
        records: vec![ValidatedExternalRecordInput {
            account_id: "account-hsbc-cash".to_string(),
            account_balance_delta: Some("-750.00".to_string()),
            amount_value: Some("750.00".to_string()),
            currency: Some("SGD".to_string()),
            event_type: Some("credit_card_repayment".to_string()),
            posted_on: Some("2026-06-30".to_string()),
            posting_status: Some("posted".to_string()),
            raw_json: r#"{"private":"must-not-leak"}"#.to_string(),
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
            "input-hash-identical",
            "output-hash-identical",
        )
        .expect("persist parse");

    let count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM external_records WHERE stable_record_key = 'hsbc-card-payment'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 1);

    let divergence_items: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM review_items WHERE external_record_id = 'record-hsbc-cash' AND reason_code = 'reparse_divergence'",
            [],
            |row| row.get(0),
        )
        .expect("divergence items");
    assert_eq!(divergence_items, 0);

    let (audit_action, source_ref): (String, String) = store
        .connection
        .query_row(
            "SELECT action, source_ref FROM audit_log \
             WHERE entity_type = 'external_record' AND entity_id = 'record-hsbc-cash' \
               AND action = 'reparse_skipped_committed_record'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("audit log entry");
    assert_eq!(audit_action, "reparse_skipped_committed_record");
    assert!(source_ref.starts_with("hsbc-card-payment:"));
}

#[test]
fn committed_record_reparse_with_divergent_output_attaches_review_item_to_committed_record() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
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
             VALUES ('job-reparse-div-1', 'parse_document', 'running', 'document-hsbc-june', \
                     '{\"documentId\":\"document-hsbc-june\",\"logicalRunKey\":\"run-div-1\"}', 'test-worker', datetime('now', '+5 minutes'))",
            [],
        )
        .expect("seed parse job");
    let claim = ParseDocumentClaim {
        claim_token: "test-worker".to_string(),
        document_id: "document-hsbc-june".to_string(),
        job_id: "job-reparse-div-1".to_string(),
        logical_run_key: "run-div-1".to_string(),
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
            "input-hash-div-1",
            "output-hash-div-1",
        )
        .expect("persist parse");

    let count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM external_records WHERE stable_record_key = 'hsbc-card-payment'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 1);

    let (divergence_items, item_status): (i64, String) = store
        .connection
        .query_row(
            "SELECT count(*), status FROM review_items \
             WHERE external_record_id = 'record-hsbc-cash' AND reason_code = 'reparse_divergence'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("divergence items");
    assert_eq!(divergence_items, 1);
    assert_eq!(item_status, "open");

    // Reparse a second time with divergence: deduplication check
    store
        .connection
        .execute(
            "INSERT INTO jobs(id, job_type, status, related_source_document_id, input_json, lease_owner, lease_until) \
             VALUES ('job-reparse-div-2', 'parse_document', 'running', 'document-hsbc-june', \
                     '{\"documentId\":\"document-hsbc-june\",\"logicalRunKey\":\"run-div-2\"}', 'test-worker', datetime('now', '+5 minutes'))",
            [],
        )
        .expect("seed second parse job");
    let claim2 = ParseDocumentClaim {
        claim_token: "test-worker".to_string(),
        document_id: "document-hsbc-june".to_string(),
        job_id: "job-reparse-div-2".to_string(),
        logical_run_key: "run-div-2".to_string(),
    };
    store
        .persist_validated_structured_parse_for_claimed_job(
            "document-hsbc-june",
            &parse_input,
            &claim2,
            "input-hash-div-2",
            "output-hash-div-2",
        )
        .expect("persist parse again");

    let open_items_count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM review_items \
             WHERE external_record_id = 'record-hsbc-cash' AND reason_code = 'reparse_divergence' AND status = 'open'",
            [],
            |row| row.get(0),
        )
        .expect("deduplicated open items count");
    assert_eq!(open_items_count, 1);
}

#[test]
fn commit_prepared_review_group_rejects_record_with_committed_sibling() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);

    // Mark v1 record-hsbc-cash as committed
    store
        .connection
        .execute(
            "UPDATE external_records SET status = 'committed' WHERE id = 'record-hsbc-cash'",
            [],
        )
        .expect("mark v1 committed");

    // Manually insert dirty v2 review record with the same stable_record_key as the committed record
    // Temporarily drop the trigger to simulate pre-migration dirty DB state
    store
        .connection
        .execute(
            "DROP TRIGGER external_record_committed_version_is_final",
            [],
        )
        .expect("drop trigger to simulate pre-migration dirty state");

    store
        .connection
        .execute(
            "INSERT INTO external_records( \
               id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
               status, record_type, event_type, posted_on, amount_value, currency, \
               account_balance_delta, raw_json, validation_json \
             ) VALUES ('record-hsbc-cash-v2', 'parse-document-hsbc-june', 'document-hsbc-june', \
                       'account-hsbc-cash', 'hsbc-card-payment', 2, 'review', 'transaction', \
                       'credit_card_repayment', '2026-06-30', '750.00', 'SGD', '-750.00', \
                       '{\"private\":\"must-not-leak\"}', '{\"internal\":true}')",
            [],
        )
        .expect("insert dirty v2");
    store
        .connection
        .execute(
            "INSERT INTO review_items(id, external_record_id, reason_code, status) \
             VALUES ('review-record-hsbc-cash-v2', 'record-hsbc-cash-v2', 'possible_card_repayment', 'open')",
            [],
        )
        .expect("seed review item for v2");

    store
        .connection
        .execute(
            "CREATE TRIGGER external_record_committed_version_is_final \
             BEFORE INSERT ON external_records \
             WHEN EXISTS( \
               SELECT 1 FROM external_records \
               WHERE stable_record_key = NEW.stable_record_key AND status = 'committed' \
             ) \
             BEGIN \
               SELECT RAISE(ABORT, 'committed stable_record_key cannot gain a new version'); \
             END;",
            [],
        )
        .expect("re-create trigger");

    let mut event = prepared_repayment();
    event.source_record_ids = vec![
        "record-dbs-card".to_owned(),
        "record-hsbc-cash-v2".to_owned(),
    ];

    // Accept relationship between dirty v2 and record-dbs-card
    store
        .accept_review_relationship(
            "review-record-hsbc-cash-v2",
            2,
            "record-dbs-card",
            1,
            &event,
        )
        .expect("accept relationship with dirty v2");

    let job = store
        .enqueue_commit_review_batch(&[
            "review-record-hsbc-cash-v2".to_owned(),
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
    assert_eq!(groups.len(), 1);

    let outcome = store
        .commit_prepared_review_group(&claimed, &groups[0], &event)
        .expect("attempt commit of group with committed sibling");

    assert_eq!(outcome.status, ReviewBatchGroupStatus::Stale);
    assert_eq!(outcome.reason.as_deref(), Some("committed_sibling_exists"));

    // Confirm no ledger events or match edges were created
    let event_count: i64 = store
        .connection
        .query_row("SELECT count(*) FROM ledger_events", [], |row| row.get(0))
        .expect("count ledger events");
    assert_eq!(event_count, 0);

    let match_edge_count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM match_edges WHERE external_record_id = 'record-hsbc-cash-v2'",
            [],
            |row| row.get(0),
        )
        .expect("count match edges for v2");
    assert_eq!(match_edge_count, 0);
}

#[test]
fn schema_migration_13_blocks_direct_insert_for_committed_stable_record_key() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let store = open_store(root.path());

    // Insert a committed record
    store
        .connection
        .execute(
            "INSERT INTO source_documents(id, money_source_id, file_sha256, semantic_document_key, original_filename, mime_type, byte_size, file_state) \
             VALUES ('doc-test', 'source-dbs', 'sha-doc-test', 'doc-test-sem', 'doc.csv', 'text/csv', 10, 'missing')",
            [],
        )
        .expect("seed doc");
    store
        .connection
        .execute(
            "INSERT INTO parse_runs(id, source_document_id, normalization_profile_id, logical_run_key, profile_json, input_hash, status) \
             VALUES ('parse-test', 'doc-test', 'norm-1', 'run-1', '{}', 'hash', 'validated')",
            [],
        )
        .expect("seed parse run");
    store
        .connection
        .execute(
            "INSERT INTO external_records(id, parse_run_id, source_document_id, stable_record_key, version, status, record_type, raw_json, validation_json) \
             VALUES ('rec-committed', 'parse-test', 'doc-test', 'key-terminal', 1, 'committed', 'transaction', '{}', '{}')",
            [],
        )
        .expect("insert committed record");

    // Attempt direct insert with same stable_record_key
    let err = store
        .connection
        .execute(
            "INSERT INTO external_records(id, parse_run_id, source_document_id, stable_record_key, version, status, record_type, raw_json, validation_json) \
             VALUES ('rec-v2', 'parse-test', 'doc-test', 'key-terminal', 2, 'staged', 'transaction', '{}', '{}')",
            [],
        )
        .expect_err("trigger should abort insert of new version for committed record");

    assert!(
        err.to_string()
            .contains("committed stable_record_key cannot gain a new version"),
        "unexpected error: {err}"
    );
}

#[test]
fn reparse_with_mixed_committed_and_uncommitted_records_stages_only_uncommitted_records() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());

    // Create a document and seed accounts
    store
        .connection
        .execute(
            "INSERT INTO accounts(id, money_source_id, provider_key, provider_account_id, account_type, display_name, currency, status) \
             VALUES ('account-mixed', 'source-dbs', 'dbs', 'mixed-1', 'deposit_account', 'DBS Mixed', 'SGD', 'confirmed')",
            [],
        )
        .expect("seed account");
    store
        .connection
        .execute(
            "INSERT INTO source_documents(id, money_source_id, file_sha256, semantic_document_key, original_filename, mime_type, byte_size, file_state) \
             VALUES ('doc-mixed', 'source-dbs', 'sha-doc-mixed', 'doc-mixed-sem', 'mixed.csv', 'text/csv', 10, 'missing')",
            [],
        )
        .expect("seed doc");
    store
        .connection
        .execute(
            "INSERT INTO parse_runs(id, source_document_id, normalization_profile_id, logical_run_key, profile_json, input_hash, status) \
             VALUES ('parse-mixed-1', 'doc-mixed', 'norm-mixed', 'run-mixed-1', '{}', 'hash', 'validated')",
            [],
        )
        .expect("seed parse run");

    // Seed record-1 as committed
    store
        .connection
        .execute(
            "INSERT INTO external_records(id, parse_run_id, source_document_id, account_id, stable_record_key, version, status, record_type, amount_value, currency, raw_json, validation_json) \
             VALUES ('rec-committed-1', 'parse-mixed-1', 'doc-mixed', 'account-mixed', 'key-committed', 1, 'committed', 'transaction', '100.00', 'SGD', '{}', '{}')",
            [],
        )
        .expect("seed committed record");

    // Seed record-2 as staged
    store
        .connection
        .execute(
            "INSERT INTO external_records(id, parse_run_id, source_document_id, account_id, stable_record_key, version, status, record_type, amount_value, currency, raw_json, validation_json) \
             VALUES ('rec-uncommitted-1', 'parse-mixed-1', 'doc-mixed', 'account-mixed', 'key-uncommitted', 1, 'staged', 'transaction', '200.00', 'SGD', '{}', '{}')",
            [],
        )
        .expect("seed uncommitted record");

    // Start a reparse job
    store
        .connection
        .execute(
            "INSERT INTO jobs(id, job_type, status, related_source_document_id, input_json, lease_owner, lease_until) \
             VALUES ('job-reparse-mixed', 'parse_document', 'running', 'doc-mixed', \
                     '{\"documentId\":\"doc-mixed\",\"logicalRunKey\":\"run-mixed-2\"}', 'test-worker', datetime('now', '+5 minutes'))",
            [],
        )
        .expect("seed mixed parse job");
    let claim = ParseDocumentClaim {
        claim_token: "test-worker".to_string(),
        document_id: "doc-mixed".to_string(),
        job_id: "job-reparse-mixed".to_string(),
        logical_run_key: "run-mixed-2".to_string(),
    };

    let parse_input = ValidatedStructuredParseInput {
        normalization_profile_id: "norm-mixed".to_string(),
        profile_json: "{}".to_string(),
        records: vec![
            ValidatedExternalRecordInput {
                account_id: "account-mixed".to_string(),
                account_balance_delta: None,
                amount_value: Some("100.00".to_string()),
                currency: Some("SGD".to_string()),
                event_type: None,
                posted_on: None,
                posting_status: Some("posted".to_string()),
                raw_json: "{}".to_string(),
                record_type: "transaction".to_string(),
                stable_record_key: "key-committed".to_string(),
                validation_json:
                    r#"{"schemaValid":true,"rawGrounded":true,"deterministicValidationPassed":true}"#
                        .to_string(),
            },
            ValidatedExternalRecordInput {
                account_id: "account-mixed".to_string(),
                account_balance_delta: None,
                amount_value: Some("250.00".to_string()),
                currency: Some("SGD".to_string()),
                event_type: None,
                posted_on: None,
                posting_status: Some("posted".to_string()),
                raw_json: "{}".to_string(),
                record_type: "transaction".to_string(),
                stable_record_key: "key-uncommitted".to_string(),
                validation_json:
                    r#"{"schemaValid":true,"rawGrounded":true,"deterministicValidationPassed":true}"#
                        .to_string(),
            },
        ],
    };

    store
        .persist_validated_structured_parse_for_claimed_job(
            "doc-mixed",
            &parse_input,
            &claim,
            "input-hash-mixed-2",
            "output-hash-mixed-2",
        )
        .expect("persist mixed reparse");

    // Check committed record: still only version 1, committed
    let committed_versions: Vec<(i64, String)> = store
        .connection
        .prepare("SELECT version, status FROM external_records WHERE stable_record_key = 'key-committed'")
        .expect("prepare")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");
    assert_eq!(committed_versions, vec![(1, "committed".to_string())]);

    // Check uncommitted record: version 1 is superseded, version 2 is staged
    let uncommitted_versions: Vec<(i64, String)> = store
        .connection
        .prepare("SELECT version, status FROM external_records WHERE stable_record_key = 'key-uncommitted' ORDER BY version")
        .expect("prepare")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");
    assert_eq!(
        uncommitted_versions,
        vec![(1, "superseded".to_string()), (2, "staged".to_string())]
    );
}

#[test]
fn schema_migration_13_repairs_preexisting_dirty_records_items_and_relationships() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let database_path = root.path().join(DATABASE_FILE_NAME);
    let mut connection = open_encrypted_database(
        &database_path,
        &KEY,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .expect("open encrypted database");

    // Apply migrations 1 through 12 only
    apply_migration_set(&mut connection, &MIGRATIONS[..12]).expect("apply migrations 1..12");

    // Seed money source and account
    connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES ('source-hsbc', 'hsbc', 'HSBC', 'bank')",
            [],
        )
        .expect("seed source");
    connection
        .execute(
            "INSERT INTO accounts(id, money_source_id, provider_key, provider_account_id, account_type, display_name, currency, status) \
             VALUES ('acc-1', 'source-hsbc', 'hsbc', 'cash-1', 'deposit_account', 'Everyday', 'SGD', 'confirmed'), \
                    ('acc-2', 'source-hsbc', 'hsbc', 'cash-2', 'deposit_account', 'Savings', 'SGD', 'confirmed')",
            [],
        )
        .expect("seed accounts");
    connection
        .execute(
            "INSERT INTO source_documents(id, money_source_id, file_sha256, semantic_document_key, original_filename, mime_type, byte_size, file_state) \
             VALUES ('doc-1', 'source-hsbc', 'sha-doc-1', 'sem-doc-1', 'doc.csv', 'text/csv', 10, 'missing')",
            [],
        )
        .expect("seed doc");
    connection
        .execute(
            "INSERT INTO parse_runs(id, source_document_id, normalization_profile_id, logical_run_key, profile_json, input_hash, status) \
             VALUES ('parse-1', 'doc-1', 'norm-1', 'run-1', '{}', 'hash', 'validated')",
            [],
        )
        .expect("seed parse run");

    // Seed committed v1
    connection
        .execute(
            "INSERT INTO external_records(id, parse_run_id, source_document_id, account_id, stable_record_key, version, status, record_type, raw_json, validation_json) \
             VALUES ('rec-v1', 'parse-1', 'doc-1', 'acc-1', 'stable-key-1', 1, 'committed', 'transaction', '{}', '{}')",
            [],
        )
        .expect("seed committed v1");

    // Seed dirty staged v2 with same stable_record_key
    connection
        .execute(
            "INSERT INTO external_records(id, parse_run_id, source_document_id, account_id, stable_record_key, version, status, record_type, raw_json, validation_json) \
             VALUES ('rec-v2', 'parse-1', 'doc-1', 'acc-1', 'stable-key-1', 2, 'staged', 'transaction', '{}', '{}')",
            [],
        )
        .expect("seed dirty staged v2");

    // Seed another staged record to form a relationship
    connection
        .execute(
            "INSERT INTO external_records(id, parse_run_id, source_document_id, account_id, stable_record_key, version, status, record_type, raw_json, validation_json) \
             VALUES ('rec-other', 'parse-1', 'doc-1', 'acc-2', 'stable-key-other', 1, 'staged', 'transaction', '{}', '{}')",
            [],
        )
        .expect("seed other staged record");

    // Seed open review items
    connection
        .execute(
            "INSERT INTO review_items(id, external_record_id, reason_code, status) \
             VALUES ('rev-v2', 'rec-v2', 'test_reason', 'open'), \
                    ('rev-other', 'rec-other', 'test_reason', 'open')",
            [],
        )
        .expect("seed review items");

    // Seed accepted review relationship involving rec-v2
    connection
        .execute(
            "INSERT INTO review_relationships(id, event_type, first_external_record_id, second_external_record_id, first_review_item_id, second_review_item_id, allocation_value, unit, status) \
             VALUES ('rel-1', 'same_currency_transfer', 'rec-other', 'rec-v2', 'rev-other', 'rev-v2', '100.00', 'SGD', 'accepted')",
            [],
        )
        .expect("seed relationship");

    // Now apply migration 13!
    apply_migration_set(&mut connection, &MIGRATIONS[12..13]).expect("apply migration 13");

    // Assert rec-v2 was superseded
    let v2_status: String = connection
        .query_row(
            "SELECT status FROM external_records WHERE id = 'rec-v2'",
            [],
            |row| row.get(0),
        )
        .expect("read v2 status");
    assert_eq!(v2_status, "superseded");

    // Assert rev-v2 was resolved
    let rev_v2_status: String = connection
        .query_row(
            "SELECT status FROM review_items WHERE id = 'rev-v2'",
            [],
            |row| row.get(0),
        )
        .expect("read rev-v2 status");
    assert_eq!(rev_v2_status, "resolved");

    // Assert rel-1 was invalidated
    let rel_status: String = connection
        .query_row(
            "SELECT status FROM review_relationships WHERE id = 'rel-1'",
            [],
            |row| row.get(0),
        )
        .expect("read rel status");
    assert_eq!(rel_status, "invalidated");

    // Assert other record/item was untouched
    let other_status: String = connection
        .query_row(
            "SELECT status FROM external_records WHERE id = 'rec-other'",
            [],
            |row| row.get(0),
        )
        .expect("read other status");
    assert_eq!(other_status, "staged");
    let other_rev_status: String = connection
        .query_row(
            "SELECT status FROM review_items WHERE id = 'rev-other'",
            [],
            |row| row.get(0),
        )
        .expect("read other rev status");
    assert_eq!(other_rev_status, "open");
}
