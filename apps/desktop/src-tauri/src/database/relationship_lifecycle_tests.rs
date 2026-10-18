use super::tests::open_store;
use super::*;

/// One confirmed HSBC cash account, one confirmed HSBC savings account, and one
/// still-unconfirmed DBS candidate account. The paired records live on the cash
/// and candidate accounts, so dismissing the candidate retires exactly one side
/// of the accepted pair.
///
/// Two further rows pin what must never move: an independent peer pair on other
/// records, and one committed pair. Committed rows are immutable in the schema,
/// so the pair also proves the invalidation predicate never reaches history.
fn seed_relationship_fixture(store: &mut ManualImportStore) {
    store
        .seed_money_source("source-hsbc", "hsbc", "HSBC", "bank")
        .expect("seed HSBC source");
    store
        .connection
        .execute(
            "INSERT INTO accounts( \
               id, money_source_id, provider_key, provider_account_id, account_type, \
               display_name, currency, status \
             ) VALUES \
               ('account-hsbc-cash', 'source-hsbc', 'hsbc', 'cash-1', 'deposit_account', \
                'HSBC Everyday', 'SGD', 'confirmed'), \
               ('account-hsbc-savings', 'source-hsbc', 'hsbc', 'savings-1', 'deposit_account', \
                'HSBC Savings', 'SGD', 'confirmed'), \
               ('account-dbs-candidate', 'source-dbs', 'dbs', 'candidate-1', 'deposit_account', \
                'DBS New', 'SGD', 'candidate')",
            [],
        )
        .expect("seed accounts");
    store
        .connection
        .execute(
            "INSERT INTO instruments(id, instrument_type, symbol, currency, display_name) \
             VALUES ('instrument-sgd', 'fiat_currency', 'SGD', 'SGD', 'Singapore Dollar')",
            [],
        )
        .expect("seed instrument");
    for (document_id, source_id, sha, semantic_key) in [
        (
            "document-hsbc-august",
            "source-hsbc",
            "h".repeat(64),
            "hsbc:cash:2026-08",
        ),
        (
            "document-dbs-august",
            "source-dbs",
            "d".repeat(64),
            "dbs:new:2026-08",
        ),
    ] {
        store
            .connection
            .execute(
                "INSERT INTO source_documents( \
                   id, money_source_id, file_sha256, semantic_document_key, \
                   original_filename, mime_type, byte_size, file_state \
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'text/csv', 1, 'missing')",
                params![
                    document_id,
                    source_id,
                    sha,
                    semantic_key,
                    format!("{document_id}.csv")
                ],
            )
            .expect("seed statement document");
        store
            .connection
            .execute(
                "INSERT INTO parse_runs( \
                   id, source_document_id, normalization_profile_id, logical_run_key, profile_json, input_hash, status \
                 ) VALUES (?1, ?2, 'synthetic-review-v1', ?1, '{}', '', 'validated')",
                params![format!("parse-{document_id}"), document_id],
            )
            .expect("seed parse run");
    }
    for (id, document_id, account_id, stable_key, status, posted_on, delta) in [
        (
            "record-hsbc-out",
            "document-hsbc-august",
            "account-hsbc-cash",
            "hsbc-out-1",
            "review",
            "2026-08-02",
            "-250.00",
        ),
        (
            "record-dbs-in",
            "document-dbs-august",
            "account-dbs-candidate",
            "dbs-in-1",
            "review",
            "2026-08-03",
            "250.00",
        ),
        (
            "record-hsbc-peer-a",
            "document-hsbc-august",
            "account-hsbc-cash",
            "hsbc-peer-a",
            "review",
            "2026-08-06",
            "-15.00",
        ),
        (
            "record-hsbc-peer-b",
            "document-hsbc-august",
            "account-hsbc-savings",
            "hsbc-peer-b",
            "review",
            "2026-08-06",
            "15.00",
        ),
        (
            "record-hsbc-committed-a",
            "document-hsbc-august",
            "account-hsbc-cash",
            "hsbc-committed-a",
            "committed",
            "2026-08-07",
            "-30.00",
        ),
        (
            "record-hsbc-committed-b",
            "document-hsbc-august",
            "account-hsbc-savings",
            "hsbc-committed-b",
            "committed",
            "2026-08-07",
            "30.00",
        ),
    ] {
        store
            .connection
            .execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                   status, record_type, event_type, posted_on, amount_value, currency, \
                   account_balance_delta, raw_json, validation_json \
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, 'transaction', \
                           'same_currency_transfer', ?7, '250.00', 'SGD', ?8, '{}', '{}')",
                params![
                    id,
                    format!("parse-{document_id}"),
                    document_id,
                    account_id,
                    stable_key,
                    status,
                    posted_on,
                    delta
                ],
            )
            .expect("seed fixture record");
        store
            .connection
            .execute(
                "INSERT INTO review_items(id, external_record_id, reason_code, status) \
                 VALUES (?1, ?2, 'possible_transfer', 'open')",
                params![format!("review-{id}"), id],
            )
            .expect("seed fixture review item");
    }
    store
        .connection
        .execute(
            "INSERT INTO review_relationships( \
               id, event_type, first_external_record_id, second_external_record_id, \
               first_review_item_id, second_review_item_id, allocation_value, unit, status \
             ) VALUES \
               ('relationship-peer', 'same_currency_transfer', 'record-hsbc-peer-a', \
                'record-hsbc-peer-b', 'review-record-hsbc-peer-a', 'review-record-hsbc-peer-b', \
                '15.00', 'SGD', 'accepted'), \
               ('relationship-committed', 'same_currency_transfer', 'record-hsbc-committed-a', \
                'record-hsbc-committed-b', 'review-record-hsbc-committed-a', \
                'review-record-hsbc-committed-b', '30.00', 'SGD', 'committed')",
            [],
        )
        .expect("seed peer and committed relationships");
}

/// The transfer the fixture's `record-hsbc-out` and `record-dbs-in` describe.
fn prepared_fixture_transfer() -> CorePreparedReviewEvent {
    CorePreparedReviewEvent {
        event_class: "posting".to_owned(),
        event_date: "2026-08-02".to_owned(),
        event_type: "same_currency_transfer".to_owned(),
        source_record_ids: vec!["record-dbs-in".to_owned(), "record-hsbc-out".to_owned()],
        legs: vec![
            CoreReviewLeg {
                account_id: "account-dbs-candidate".to_owned(),
                instrument_id: "instrument-sgd".to_owned(),
                currency: "SGD".to_owned(),
                amount_value: "250.00".to_owned(),
            },
            CoreReviewLeg {
                account_id: "account-hsbc-cash".to_owned(),
                instrument_id: "instrument-sgd".to_owned(),
                currency: "SGD".to_owned(),
                amount_value: "-250.00".to_owned(),
            },
        ],
        spending: false,
    }
}

fn accept_fixture_transfer(store: &mut ManualImportStore) {
    let accepted = store
        .accept_review_relationship(
            "review-record-hsbc-out",
            1,
            "record-dbs-in",
            1,
            &prepared_fixture_transfer(),
        )
        .expect("accept the fixture transfer");
    assert_eq!(
        (accepted.status, accepted.reason),
        (ReviewMutationStatus::RelationshipAccepted, None),
    );
}

fn relationship_status(store: &ManualImportStore, id: &str) -> String {
    store
        .connection
        .query_row(
            "SELECT status FROM review_relationships WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .expect("read relationship status")
}

fn record_status(store: &ManualImportStore, id: &str) -> String {
    store
        .connection
        .query_row(
            "SELECT status FROM external_records WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .expect("read record status")
}

/// Q07 (BRAWUKA-904): a reparse retires the live version of a record, so every
/// accepted relationship naming that record must be invalidated in the same
/// transaction — and nothing else may move.
#[test]
fn reparse_invalidates_the_retired_accepted_pair_and_keeps_peers_and_history() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_relationship_fixture(&mut store);
    accept_fixture_transfer(&mut store);

    store
        .connection
        .execute(
            "INSERT INTO jobs(id, job_type, status, related_source_document_id, input_json, lease_owner, lease_until) \
             VALUES ('job-reparse-lifecycle', 'parse_document', 'running', 'document-hsbc-august', \
                     '{\"documentId\":\"document-hsbc-august\",\"logicalRunKey\":\"run-reparse-lifecycle\"}', \
                     'test-worker', datetime('now', '+5 minutes'))",
            [],
        )
        .expect("seed reparse job");
    let claim = ParseDocumentClaim {
        claim_token: "test-worker".to_owned(),
        document_id: "document-hsbc-august".to_owned(),
        job_id: "job-reparse-lifecycle".to_owned(),
        logical_run_key: "run-reparse-lifecycle".to_owned(),
    };
    let parse_input = ValidatedStructuredParseInput {
        normalization_profile_id: "synthetic-review-v1".to_owned(),
        profile_json: "{}".to_owned(),
        records: vec![ValidatedExternalRecordInput {
            account_id: "account-hsbc-cash".to_owned(),
            account_balance_delta: Some("-250.00".to_owned()),
            amount_value: Some("250.00".to_owned()),
            currency: Some("SGD".to_owned()),
            event_type: Some("same_currency_transfer".to_owned()),
            posted_on: Some("2026-08-02".to_owned()),
            posting_status: Some("posted".to_owned()),
            raw_json: "{}".to_owned(),
            record_type: "transaction".to_owned(),
            stable_record_key: "hsbc-out-1".to_owned(),
            validation_json:
                r#"{"schemaValid":true,"rawGrounded":true,"deterministicValidationPassed":true}"#
                    .to_owned(),
        }],
    };
    store
        .persist_validated_structured_parse_for_claimed_job(
            "document-hsbc-august",
            &parse_input,
            &claim,
            "input-hash-lifecycle",
            "output-hash-lifecycle",
        )
        .expect("reparse the document");

    assert_eq!(record_status(&store, "record-hsbc-out"), "superseded");
    let affected: String = store
        .connection
        .query_row(
            "SELECT status FROM review_relationships \
             WHERE first_external_record_id = 'record-hsbc-out' \
                OR second_external_record_id = 'record-hsbc-out'",
            [],
            |row| row.get(0),
        )
        .expect("read the retired pair");
    assert_eq!(
        affected, "invalidated",
        "the accepted pair whose side the reparse retired must be invalidated",
    );

    assert_eq!(relationship_status(&store, "relationship-peer"), "accepted");
    assert_eq!(record_status(&store, "record-hsbc-peer-a"), "review");
    assert_eq!(record_status(&store, "record-hsbc-peer-b"), "review");
    assert_eq!(
        relationship_status(&store, "relationship-committed"),
        "committed"
    );
    assert_eq!(
        record_status(&store, "record-hsbc-committed-a"),
        "committed"
    );
    assert_eq!(
        record_status(&store, "record-hsbc-committed-b"),
        "committed"
    );
}

/// The same contract for a candidate-account dismissal: the retirement of the
/// account's live records and the invalidation of their accepted pairs happen
/// in one transaction, and untouched pairs keep their state.
#[test]
fn dismissing_a_candidate_account_invalidates_only_its_accepted_pairs() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_relationship_fixture(&mut store);
    accept_fixture_transfer(&mut store);

    let proposal_version = store
        .list_account_confirmation_prompts()
        .expect("read candidate proposal")
        .into_iter()
        .find(|prompt| prompt.money_source_id == "source-dbs")
        .expect("DBS candidate prompt")
        .proposal_version;
    assert_eq!(
        store
            .decide_candidate_accounts(
                "source-dbs",
                &proposal_version,
                &[CandidateAccountDecisionInput {
                    account_id: "account-dbs-candidate".to_owned(),
                    action: CandidateAccountDecision::Dismiss,
                }],
                "audit-dismiss-lifecycle",
            )
            .expect("dismiss the candidate account")
            .status,
        AccountConfirmationStatus::Updated,
    );

    assert_eq!(record_status(&store, "record-dbs-in"), "removed");
    let affected: String = store
        .connection
        .query_row(
            "SELECT status FROM review_relationships \
             WHERE first_external_record_id = 'record-dbs-in' \
                OR second_external_record_id = 'record-dbs-in'",
            [],
            |row| row.get(0),
        )
        .expect("read the dismissed pair");
    assert_eq!(
        affected, "invalidated",
        "the accepted pair whose side the dismissal retired must be invalidated",
    );

    assert_eq!(record_status(&store, "record-hsbc-out"), "review");
    assert_eq!(relationship_status(&store, "relationship-peer"), "accepted");
    assert_eq!(record_status(&store, "record-hsbc-peer-a"), "review");
    assert_eq!(
        relationship_status(&store, "relationship-committed"),
        "committed"
    );
    assert_eq!(
        record_status(&store, "record-hsbc-committed-a"),
        "committed"
    );
}

/// After a dismissed side is restored the pair can be re-paired and committed
/// exactly once: the earlier row is re-activated against the current open
/// review items, and the commit stays idempotent.
#[test]
fn a_restored_pair_is_repaired_and_commits_exactly_once() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_relationship_fixture(&mut store);
    accept_fixture_transfer(&mut store);

    let proposal_version = store
        .list_account_confirmation_prompts()
        .expect("read candidate proposal")
        .into_iter()
        .find(|prompt| prompt.money_source_id == "source-dbs")
        .expect("DBS candidate prompt")
        .proposal_version;
    store
        .decide_candidate_accounts(
            "source-dbs",
            &proposal_version,
            &[CandidateAccountDecisionInput {
                account_id: "account-dbs-candidate".to_owned(),
                action: CandidateAccountDecision::Dismiss,
            }],
            "audit-dismiss-repair",
        )
        .expect("dismiss the candidate account");
    assert_eq!(
        store
            .restore_dismissed_candidate_account("account-dbs-candidate", "audit-restore-repair")
            .expect("restore the candidate account")
            .status,
        AccountConfirmationStatus::Restored,
    );
    assert_eq!(record_status(&store, "record-dbs-in"), "review");

    // A restored account is a candidate again, so the user confirms it before
    // the pair can carry anything into the ledger.
    let restored_proposal_version = store
        .list_account_confirmation_prompts()
        .expect("read restored candidate proposal")
        .into_iter()
        .find(|prompt| prompt.money_source_id == "source-dbs")
        .expect("restored DBS candidate prompt")
        .proposal_version;
    assert_eq!(
        store
            .decide_candidate_accounts(
                "source-dbs",
                &restored_proposal_version,
                &[CandidateAccountDecisionInput {
                    account_id: "account-dbs-candidate".to_owned(),
                    action: CandidateAccountDecision::Accept,
                }],
                "audit-confirm-repair",
            )
            .expect("confirm the restored candidate account")
            .status,
        AccountConfirmationStatus::Confirmed,
    );

    let restored_item: String = store
        .connection
        .query_row(
            "SELECT id FROM review_items WHERE external_record_id = 'record-dbs-in' \
               AND status = 'open' ORDER BY id LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read the restored review item");
    let repaired = store
        .accept_review_relationship(
            "review-record-hsbc-out",
            1,
            "record-dbs-in",
            1,
            &prepared_fixture_transfer(),
        )
        .expect("re-pair the restored records");
    assert_eq!(
        (repaired.status, repaired.reason),
        (ReviewMutationStatus::RelationshipAccepted, None),
    );
    let (status, first_item, second_item): (String, String, String) = store
        .connection
        .query_row(
            "SELECT status, first_review_item_id, second_review_item_id \
             FROM review_relationships WHERE first_external_record_id = 'record-dbs-in' \
               OR second_external_record_id = 'record-dbs-in'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read the repaired pair");
    assert_eq!(status, "accepted", "re-paired rows are accepted again");
    assert!(
        [first_item.as_str(), second_item.as_str()].contains(&restored_item.as_str()),
        "the repaired row must point at the restored open review item, not the resolved one",
    );

    let job = store
        .enqueue_commit_review_batch(&[first_item, second_item])
        .expect("enqueue the repaired pair");
    let claimed = store
        .claim_review_batch(&job.job_id, "test-worker")
        .expect("claim batch")
        .expect("queued job");
    let (groups, outcomes) = store
        .prepare_commit_review_groups(&claimed)
        .expect("prepare the repaired pair");
    assert!(outcomes.is_empty(), "{outcomes:?}");
    assert_eq!(groups.len(), 1);
    assert_eq!(
        store
            .commit_prepared_review_group(&claimed, &groups[0], &prepared_fixture_transfer())
            .expect("commit the repaired pair")
            .status,
        ReviewBatchGroupStatus::Committed,
    );
    let events: i64 = store
        .connection
        .query_row("SELECT count(*) FROM ledger_events", [], |row| row.get(0))
        .expect("count ledger events");
    assert_eq!(events, 1);

    let (groups, outcomes) = store
        .prepare_commit_review_groups(&claimed)
        .expect("prepare the committed pair again");
    assert_eq!(groups.len(), 0);
    assert_eq!(
        outcomes.first().map(|outcome| outcome.status.clone()),
        Some(ReviewBatchGroupStatus::AlreadyCommitted),
    );
    let events_after: i64 = store
        .connection
        .query_row("SELECT count(*) FROM ledger_events", [], |row| row.get(0))
        .expect("count ledger events after the second attempt");
    assert_eq!(events_after, 1, "the pair commits exactly once");
}

/// A parse that fails after the retirement rolls the whole transaction back:
/// the record keeps its version, the pair keeps `accepted`, and no parse run
/// survives.
#[test]
fn a_failed_parse_rolls_back_the_retirement_and_the_invalidation() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_relationship_fixture(&mut store);
    accept_fixture_transfer(&mut store);

    store
        .connection
        .execute(
            "INSERT INTO jobs(id, job_type, status, related_source_document_id, input_json, lease_owner, lease_until) \
             VALUES ('job-reparse-rollback', 'parse_document', 'running', 'document-hsbc-august', \
                     '{\"documentId\":\"document-hsbc-august\",\"logicalRunKey\":\"run-reparse-rollback\"}', \
                     'test-worker', datetime('now', '+5 minutes'))",
            [],
        )
        .expect("seed rollback reparse job");
    let claim = ParseDocumentClaim {
        claim_token: "test-worker".to_owned(),
        document_id: "document-hsbc-august".to_owned(),
        job_id: "job-reparse-rollback".to_owned(),
        logical_run_key: "run-reparse-rollback".to_owned(),
    };
    let valid_validation_json =
        r#"{"schemaValid":true,"rawGrounded":true,"deterministicValidationPassed":true}"#;
    let parse_input = ValidatedStructuredParseInput {
        normalization_profile_id: "synthetic-review-v1".to_owned(),
        profile_json: "{}".to_owned(),
        records: vec![
            ValidatedExternalRecordInput {
                account_id: "account-hsbc-cash".to_owned(),
                account_balance_delta: Some("-250.00".to_owned()),
                amount_value: Some("250.00".to_owned()),
                currency: Some("SGD".to_owned()),
                event_type: Some("same_currency_transfer".to_owned()),
                posted_on: Some("2026-08-02".to_owned()),
                posting_status: Some("posted".to_owned()),
                raw_json: "{}".to_owned(),
                record_type: "transaction".to_owned(),
                stable_record_key: "hsbc-out-1".to_owned(),
                validation_json: valid_validation_json.to_owned(),
            },
            // The second record names an account that does not exist, so the
            // transaction fails after the first record was already retired.
            ValidatedExternalRecordInput {
                account_id: "account-missing".to_owned(),
                account_balance_delta: Some("-1.00".to_owned()),
                amount_value: Some("1.00".to_owned()),
                currency: Some("SGD".to_owned()),
                event_type: Some("same_currency_transfer".to_owned()),
                posted_on: Some("2026-08-04".to_owned()),
                posting_status: Some("posted".to_owned()),
                raw_json: "{}".to_owned(),
                record_type: "transaction".to_owned(),
                stable_record_key: "hsbc-out-2".to_owned(),
                validation_json: valid_validation_json.to_owned(),
            },
        ],
    };
    assert!(
        store
            .persist_validated_structured_parse_for_claimed_job(
                "document-hsbc-august",
                &parse_input,
                &claim,
                "input-hash-rollback",
                "output-hash-rollback",
            )
            .is_err(),
        "a parse whose record cannot be inserted must fail",
    );

    assert_eq!(record_status(&store, "record-hsbc-out"), "review");
    let affected: String = store
        .connection
        .query_row(
            "SELECT status FROM review_relationships \
             WHERE first_external_record_id = 'record-hsbc-out' \
                OR second_external_record_id = 'record-hsbc-out'",
            [],
            |row| row.get(0),
        )
        .expect("read the pair after the rollback");
    assert_eq!(affected, "accepted");
    let runs: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM parse_runs WHERE logical_run_key = 'run-reparse-rollback'",
            [],
            |row| row.get(0),
        )
        .expect("count rolled back parse runs");
    assert_eq!(runs, 0);
    let versions: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM external_records WHERE stable_record_key = 'hsbc-out-1'",
            [],
            |row| row.get(0),
        )
        .expect("count record versions after the rollback");
    assert_eq!(versions, 1);
}
