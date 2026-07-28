use super::*;

const KEY: [u8; KEY_LEN] = [0x72; KEY_LEN];

fn open_store(root: &Path) -> ManualImportStore {
    let store = ManualImportStore::open(root, Zeroizing::new(KEY)).expect("open Vault");
    store
        .connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES ('source-dbs', 'dbs', 'DBS', 'bank')",
            [],
        )
        .expect("seed source");
    store
}

fn import<'a>(
    source_path: &'a Path,
    document_id: &'a str,
    audit_id: &'a str,
) -> SourceDocumentImport<'a> {
    SourceDocumentImport {
        audit_actor: "user",
        audit_id,
        audit_policy_version: "manual-import-v1",
        audit_reason: "manual_import",
        document_id,
        mime_type: "application/pdf",
        original_filename: "statement.pdf",
        source_path,
    }
}

#[test]
fn migrates_legacy_queued_and_expired_parse_jobs_to_claimable_logical_runs() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let database_path = root.path().join(DATABASE_FILE_NAME);
    let mut connection = open_encrypted_database(
        &database_path,
        &KEY,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .expect("open legacy database");
    apply_migration_set(&mut connection, &MIGRATIONS[..8]).expect("apply v8 schema");
    connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES ('source-legacy', 'dbs', 'DBS', 'bank')",
            [],
        )
        .expect("seed legacy source");
    for document_id in ["document-queued", "document-expired"] {
        connection
            .execute(
                "INSERT INTO source_documents( \
                   id, file_sha256, original_filename, mime_type, byte_size, file_state \
                 ) VALUES (?1, ?2, 'statement.pdf', 'application/pdf', 1, 'missing')",
                params![document_id, format!("{document_id:0<64}")],
            )
            .expect("seed legacy document");
    }
    connection
        .execute(
            "INSERT INTO accounts( \
               id, money_source_id, provider_key, provider_account_id, account_type, \
               display_name, status \
             ) VALUES ('account-legacy', 'source-legacy', 'dbs', 'checking-legacy', \
                       'deposit_account', 'Legacy checking', 'confirmed')",
            [],
        )
        .expect("seed legacy account");
    connection
        .execute(
            "INSERT INTO parse_runs( \
               id, source_document_id, normalization_profile_id, profile_json, status \
             ) VALUES ('parse-run-legacy', 'document-queued', 'dbs-v1', '{}', 'succeeded')",
            [],
        )
        .expect("seed legacy parse run");
    connection
        .execute(
            "INSERT INTO jobs( \
               id, job_type, status, input_json, related_source_document_id, lease_owner, lease_until \
             ) VALUES \
               ('job-ingest', 'source_document_ingest', 'succeeded', '{}', \
                'document-queued', NULL, NULL), \
               ('job-queued', 'parse_document', 'queued', '{\"documentId\":\"document-queued\"}', \
                'document-queued', NULL, NULL), \
               ('job-expired', 'parse_document', 'running', '{\"documentId\":\"document-expired\"}', \
                'document-expired', 'v8-worker', datetime('now', '-1 second'))",
            [],
        )
        .expect("seed legacy parse jobs");
    apply_migration_set(&mut connection, &MIGRATIONS[8..]).expect("apply v9 hardening");
    let ingest_jobs: i64 = connection
        .query_row(
            "SELECT count(*) FROM jobs WHERE job_type = 'source_document_ingest'",
            [],
            |row| row.get(0),
        )
        .expect("count retired ingest jobs");
    assert_eq!(ingest_jobs, 0);
    connection
        .execute(
            "INSERT INTO accounts( \
               id, money_source_id, provider_key, provider_account_id, account_type, \
               display_name, status, merged_into_account_id \
             ) VALUES ('account-merged', 'source-legacy', 'dbs', 'checking-merged', \
                       'deposit_account', 'Merged checking', 'merged', 'account-legacy')",
            [],
        )
        .expect("insert account against rebuilt self reference");
    connection
        .execute(
            "INSERT INTO parse_runs( \
               id, source_document_id, normalization_profile_id, logical_run_key, profile_json, \
               input_hash, status \
             ) VALUES ('parse-run-new', 'document-queued', 'dbs-v1', 'manual-reparse', '{}', \
                       '', 'succeeded')",
            [],
        )
        .expect("insert parse run against rebuilt table");
    let foreign_key_violations: i64 = connection
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .expect("check upgraded foreign keys");
    assert_eq!(foreign_key_violations, 0);
    drop(connection);

    let mut store = ManualImportStore::open_existing(root.path(), Zeroizing::new(KEY))
        .expect("open upgraded Vault");
    let jobs = store
        .queued_parse_document_jobs()
        .expect("recover queued parse jobs");
    assert_eq!(jobs.len(), 2);
    for job in jobs {
        assert_eq!(job.logical_run_key, job.job_id);
        assert!(
            store
                .start_parse_document_job(&job)
                .expect("claim upgraded job")
                .is_some()
        );
    }
}

#[test]
fn reopen_requeues_an_unexpired_commit_batch() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let store = open_store(root.path());
    store
        .connection
        .execute(
            "INSERT INTO jobs( \
               id, job_type, status, input_json, lease_owner, lease_until \
             ) VALUES ( \
               'job-commit-running', 'commit_review_batch', 'running', \
               '{\"reviewItemIds\":[\"review-1\"]}', 'old-process', datetime('now', '+5 minutes') \
             )",
            [],
        )
        .expect("seed interrupted commit batch");
    drop(store);

    let reopened =
        ManualImportStore::open_existing(root.path(), Zeroizing::new(KEY)).expect("reopen Vault");
    let state: (String, Option<String>, Option<String>) = reopened
        .connection
        .query_row(
            "SELECT status, lease_owner, lease_until FROM jobs WHERE id = 'job-commit-running'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read recovered commit batch");
    assert_eq!(state, ("queued".to_owned(), None, None));
}

#[test]
fn reopen_requeues_an_unexpired_reconcile_job() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF reconcile recovery").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import(&source_path, "document-reconcile", "audit-reconcile"),
            None,
        )
        .expect("import source");
    store
        .connection
        .execute(
            "INSERT INTO jobs( \
               id, job_type, status, input_json, related_source_document_id, lease_owner, lease_until \
             ) VALUES ( \
               'job-reconcile-running', 'reconcile_document', 'running', '{}', \
               'document-reconcile', 'old-process', datetime('now', '+5 minutes') \
             )",
            [],
        )
        .expect("seed interrupted reconcile job");
    drop(store);

    let mut reopened =
        ManualImportStore::open_existing(root.path(), Zeroizing::new(KEY)).expect("reopen Vault");
    assert_eq!(
        reopened
            .queued_reconcile_document_ids()
            .expect("read recovered reconcile jobs"),
        vec!["document-reconcile".to_owned()]
    );
}

#[test]
fn stale_capture_plan_reports_an_available_concurrent_import_as_already_present() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF concurrent capture").expect("write fixture");
    let mut first = open_store(root.path());
    let mut second = ManualImportStore::open_existing(root.path(), Zeroizing::new(KEY))
        .expect("open concurrent Vault");
    let source = ManualImportStore::prepare_source_path("application/pdf", &source_path)
        .expect("prepare source");
    let first_input = import(&source_path, "document-first", "audit-first");
    let second_input = import(&source_path, "document-second", "audit-second");
    let first_capture = match first
        .source_capture_plan(&first_input, &source, None)
        .expect("plan first capture")
    {
        SourceCapturePlan::Capture(capture) => capture,
        SourceCapturePlan::RestoreConfirmationRequired(_) => panic!("new document"),
    };
    let first_stored = first_capture
        .store_prepared(&source)
        .expect("store first envelope");
    assert!(first_stored.created);

    let second_capture = match second
        .source_capture_plan(&second_input, &source, None)
        .expect("plan second capture")
    {
        SourceCapturePlan::Capture(capture) => capture,
        SourceCapturePlan::RestoreConfirmationRequired(_) => panic!("new document"),
    };
    let second_stored = second_capture
        .store_prepared(&source)
        .expect("reuse first envelope");
    second
        .persist_captured_import(&second_input, &second_stored, None)
        .expect("persist winning import");

    let stale = first
        .persist_captured_import(&first_input, &first_stored, None)
        .expect("persist stale capture plan");
    assert_eq!(stale.document_id, "document-second");
    assert_eq!(stale.status, SourceDocumentImportStatus::AlreadyPresent);
    let parse_jobs: i64 = first
        .connection
        .query_row(
            "SELECT count(*) FROM jobs WHERE job_type = 'parse_document'",
            [],
            |row| row.get(0),
        )
        .expect("count parse jobs");
    assert_eq!(parse_jobs, 1);
}

#[test]
fn stale_parse_claim_cannot_change_a_newer_attempt() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF stale claim").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import(&source_path, "document-stale", "audit-import"),
            None,
        )
        .expect("import document");
    let job = store
        .queued_parse_document_jobs()
        .expect("read parse job")
        .pop()
        .expect("queued parse job");
    let first = store
        .start_parse_document_job(&job)
        .expect("claim first attempt")
        .expect("first attempt claimed");
    store
        .connection
        .execute(
            "UPDATE jobs SET lease_until = datetime('now', '-1 second') WHERE id = ?1",
            [&first.job_id],
        )
        .expect("expire first lease");
    let retry = store
        .queued_parse_document_jobs()
        .expect("recover expired parse job")
        .pop()
        .expect("requeued parse job");
    let second = store
        .start_parse_document_job(&retry)
        .expect("claim retry")
        .expect("retry claimed");
    assert_ne!(first.claim_token, second.claim_token);

    let accounts = [TrustedAccountCandidate {
        account_id: "account-stale",
        account_type: "deposit_account",
        currency: Some("SGD"),
        display_name: "Checking",
        masked_identifier: None,
        provider_account_id: Some("checking-stale"),
    }];
    let classification = TrustedDocumentClassification {
        accounts: &accounts,
        audit_id: "audit-stale",
        document_id: "document-stale",
        document_type: Some("account_statement"),
        provider_key: "dbs",
        semantic_document_key: "dbs:checking:2026-07",
        statement_period_from: Some("2026-07-01"),
        statement_period_to: Some("2026-07-31"),
    };
    assert!(
        store
            .apply_trusted_classification_for_parse_job(&classification, &first)
            .is_err()
    );
    assert!(
        store
            .block_parse_document_job(&first, "password_required")
            .is_err()
    );
    assert!(
        store
            .fail_parse_document_job(&first, "normalizer_failed")
            .is_err()
    );
    assert!(
        store
            .finish_parse_document_job(
                &first,
                &SourceDocumentRoutingOutcome::needs_attention(
                    "document-stale",
                    "classification_uncertain"
                ),
            )
            .is_err()
    );
    let lease_owner: String = store
        .connection
        .query_row(
            "SELECT lease_owner FROM jobs WHERE id = ?1",
            [&second.job_id],
            |row| row.get(0),
        )
        .expect("newer lease remains active");
    assert_eq!(lease_owner, second.claim_token);
}

#[test]
fn failed_same_content_capture_leaves_the_file_referenced_by_the_other_import() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF same content").expect("write fixture");
    let mut first = open_store(root.path());
    let mut second = ManualImportStore::open_existing(root.path(), Zeroizing::new(KEY))
        .expect("open concurrent Vault");
    let source = ManualImportStore::prepare_source_path("application/pdf", &source_path)
        .expect("prepare source");
    let first_input = import(&source_path, "document-first", "audit-conflict");
    let second_input = import(&source_path, "document-second", "audit-second");
    let first_capture = match first
        .source_capture_plan(&first_input, &source, None)
        .expect("plan first capture")
    {
        SourceCapturePlan::Capture(capture) => capture,
        SourceCapturePlan::RestoreConfirmationRequired(_) => panic!("new document"),
    };
    let second_capture = match second
        .source_capture_plan(&second_input, &source, None)
        .expect("plan second capture")
    {
        SourceCapturePlan::Capture(capture) => capture,
        SourceCapturePlan::RestoreConfirmationRequired(_) => panic!("new document"),
    };
    let first_stored = first_capture
        .store_prepared(&source)
        .expect("store first envelope");
    let second_stored = second_capture
        .store_prepared(&source)
        .expect("reuse first envelope");
    assert!(first_stored.created);
    assert!(!second_stored.created);
    second
        .persist_captured_import(&second_input, &second_stored, None)
        .expect("persist second import");
    first
        .connection
        .execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES ('audit-conflict', 'test', 'test', 'seed', 'test', 'test', 'test')",
            [],
        )
        .expect("seed audit conflict");
    assert!(
        first
            .persist_captured_import(&first_input, &first_stored, None)
            .is_err()
    );
    assert!(root.path().join(&first_stored.encrypted_locator).exists());
    assert!(
        second
            .files
            .verifies(
                &KEY,
                &second_stored.encrypted_locator,
                &second_stored.file_sha256
            )
            .expect("referenced envelope verifies")
    );
}
