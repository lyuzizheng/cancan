use super::*;

const KEY: [u8; KEY_LEN] = [0x91; KEY_LEN];

fn open_store(root: &Path) -> ManualImportStore {
    let store = ManualImportStore::open(root, Zeroizing::new(KEY)).expect("open encrypted Vault");
    store
        .connection
        .execute(
            "INSERT OR IGNORE INTO money_sources( \
               id, provider_key, display_name, source_type \
             ) VALUES ('source-dbs', 'dbs', 'DBS', 'bank')",
            [],
        )
        .expect("seed money source");
    store
}

#[test]
fn lists_only_safe_money_source_display_fields_in_stable_order() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let store = open_store(root.path());
    store
        .seed_money_source("source-alpha", "alpha", "Alpha Bank", "bank")
        .expect("seed second source");

    assert_eq!(
        store.list_money_sources().expect("list Money Sources"),
        vec![
            MoneySourceView {
                display_name: "Alpha Bank".to_owned(),
                money_source_id: "source-alpha".to_owned(),
                source_type: "bank".to_owned(),
            },
            MoneySourceView {
                display_name: "DBS".to_owned(),
                money_source_id: "source-dbs".to_owned(),
                source_type: "bank".to_owned(),
            },
        ]
    );
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
        original_filename: "DBS-July-2026.pdf",
        source_path,
    }
}

#[test]
fn rejects_invalid_captured_pdf_and_csv_before_persistence() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    for (document_id, mime_type, filename, bytes) in [
        (
            "invalid-captured-pdf",
            "application/pdf",
            "invalid.pdf",
            b"not a PDF" as &[u8],
        ),
        (
            "invalid-captured-csv-utf8",
            "text/csv",
            "invalid-utf8.csv",
            b"column\n\xff",
        ),
    ] {
        let source_path = root.path().join(filename);
        assert!(
            store
                .register_captured_import(
                    &SourceDocumentImport {
                        audit_actor: "system",
                        audit_id: document_id,
                        audit_policy_version: "manual-import-v1",
                        audit_reason: "local_inbox_import",
                        document_id,
                        mime_type,
                        original_filename: filename,
                        source_path: &source_path,
                    },
                    Zeroizing::new(bytes.to_vec()),
                    None,
                )
                .is_err(),
            "{filename} must not be persisted"
        );
    }
    let documents: i64 = store
        .connection
        .query_row("SELECT count(*) FROM source_documents", [], |row| {
            row.get(0)
        })
        .expect("count source documents");
    assert_eq!(documents, 0);
    assert!(!root.path().join("files").exists());
}

#[test]
fn accepts_captured_pdf_with_a_pdf_header_without_parsing_it() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let source_path = root.path().join("protected.pdf");
    let outcome = store
        .register_captured_import(
            &SourceDocumentImport {
                audit_actor: "system",
                audit_id: "captured-protected-pdf",
                audit_policy_version: "manual-import-v1",
                audit_reason: "local_inbox_import",
                document_id: "captured-protected-pdf",
                mime_type: "application/pdf",
                original_filename: "protected.pdf",
                source_path: &source_path,
            },
            Zeroizing::new(b"%PDF-1.7\n/Encrypt".to_vec()),
            None,
        )
        .expect("capture protected PDF");
    assert_eq!(outcome.status, SourceDocumentImportStatus::Imported);
}

#[test]
fn source_document_pipeline_queues_reconciliation_only_after_parse_succeeds() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF synthetic statement").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import(&source_path, "document-local-inbox", "audit-import"),
            None,
        )
        .expect("import source");

    let initial_jobs = store
        .connection
        .prepare(
            "SELECT job_type, status FROM jobs \
             WHERE related_source_document_id = ?1 ORDER BY job_type",
        )
        .expect("prepare job query")
        .query_map(["document-local-inbox"], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("query jobs")
        .collect::<Result<Vec<_>, _>>()
        .expect("read jobs");
    assert_eq!(
        initial_jobs,
        vec![("parse_document".to_owned(), "queued".to_owned())]
    );

    let job = store
        .queued_parse_document_jobs()
        .expect("read queued parse job")
        .pop()
        .expect("queued parse job");
    let claim = store
        .start_parse_document_job(&job)
        .expect("start parse")
        .expect("claim parse job");
    store
        .finish_parse_document_job(
            &claim,
            &SourceDocumentRoutingOutcome {
                account_ids: vec!["account-dbs".to_owned()],
                document_id: "document-local-inbox".to_owned(),
                money_source_id: Some("source-dbs".to_owned()),
                reason: None,
                status: SourceDocumentRoutingStatus::Routed,
            },
        )
        .expect("finish parse");
    store
        .finish_parse_document_job(
            &claim,
            &SourceDocumentRoutingOutcome {
                account_ids: vec!["account-dbs".to_owned()],
                document_id: "document-local-inbox".to_owned(),
                money_source_id: Some("source-dbs".to_owned()),
                reason: None,
                status: SourceDocumentRoutingStatus::Routed,
            },
        )
        .expect_err("reject completion after the parse lease is released");
    let final_jobs = store
        .connection
        .prepare(
            "SELECT job_type, status FROM jobs \
             WHERE related_source_document_id = ?1 ORDER BY job_type",
        )
        .expect("prepare job query")
        .query_map(["document-local-inbox"], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("query jobs")
        .collect::<Result<Vec<_>, _>>()
        .expect("read jobs");
    assert_eq!(
        final_jobs,
        vec![
            ("parse_document".to_owned(), "succeeded".to_owned()),
            ("reconcile_document".to_owned(), "queued".to_owned()),
        ]
    );
}

#[test]
fn automatically_retries_failed_parse_jobs_with_the_same_logical_run_until_exhausted() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF transient sidecar failure").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(&import(&source_path, "document-retry", "audit-retry"), None)
        .expect("import source");
    let first_job = store
        .queued_parse_document_jobs()
        .expect("read initial parse job")
        .pop()
        .expect("initial parse job");

    let first_claim = store
        .start_parse_document_job(&first_job)
        .expect("start first parse")
        .expect("claim first parse");
    store
        .fail_parse_document_job(&first_claim, "normalizer_failed")
        .expect("requeue transient parse failure");
    let retry_state: (String, i64, bool) = store
        .connection
        .query_row(
            "SELECT status, attempts, \
                    result_json IS NULL AND error_json IS NULL AND blocked_reason IS NULL \
                    AND lease_owner IS NULL AND lease_until IS NULL AND finished_at IS NULL \
             FROM jobs WHERE id = ?1",
            [&first_job.job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read re-queued parse");
    assert_eq!(retry_state, ("queued".to_owned(), 1, true));
    let retry_job = store
        .queued_parse_document_jobs()
        .expect("read retry parse job")
        .pop()
        .expect("retry parse job");
    assert_eq!(retry_job.job_id, first_job.job_id);
    assert_eq!(retry_job.logical_run_key, first_job.logical_run_key);
    let retry_claim = store
        .start_parse_document_job(&retry_job)
        .expect("retry parse")
        .expect("claim retry parse");
    store
        .fail_parse_document_job(&retry_claim, "normalizer_failed")
        .expect("requeue second parse failure");
    let final_claim = store
        .start_parse_document_job(&retry_job)
        .expect("final retry")
        .expect("claim final retry");
    store
        .fail_parse_document_job(&final_claim, "normalizer_failed")
        .expect("record exhausted failure");
    let terminal: (String, i64, String) = store
        .connection
        .query_row(
            "SELECT status, attempts, blocked_reason FROM jobs WHERE id = ?1",
            [&retry_job.job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read terminal retry state");
    assert_eq!(
        terminal,
        ("failed".to_owned(), 3, "normalizer_failed".to_owned())
    );
}

#[test]
fn explicit_reparse_creates_a_new_parse_job_and_logical_run() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF explicit reparse").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import(&source_path, "document-reparse", "audit-reparse"),
            None,
        )
        .expect("import source");
    let first = store
        .queued_parse_document_jobs()
        .expect("read initial parse job")
        .pop()
        .expect("initial parse job");
    let first_claim = store
        .start_parse_document_job(&first)
        .expect("claim initial parse")
        .expect("initial parse claimed");
    store
        .finish_parse_document_job(
            &first_claim,
            &SourceDocumentRoutingOutcome::needs_attention(
                "document-reparse",
                "classification_uncertain",
            ),
        )
        .expect("finish initial parse");

    assert!(
        store
            .enqueue_source_document_pipeline("document-reparse")
            .expect("enqueue explicit reparse")
    );
    assert!(
        !store
            .enqueue_source_document_pipeline("document-reparse")
            .expect("reject duplicate active reparse")
    );
    let jobs = store
        .queued_parse_document_jobs()
        .expect("read queued parse jobs");

    assert_eq!(jobs.len(), 1);
    assert_ne!(jobs[0].logical_run_key, first.logical_run_key);
}

#[test]
fn migrates_existing_document_relationships_to_pending_semantic_identity() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let database_path = root.path().join(DATABASE_FILE_NAME);
    let mut connection = open_encrypted_database(
        &database_path,
        &KEY,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .expect("open encrypted database");
    apply_migration_set(&mut connection, &MIGRATIONS[..2]).expect("apply old schema");
    connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES ('source-dbs', 'dbs', 'DBS', 'bank')",
            [],
        )
        .expect("seed source");
    connection
        .execute(
            "INSERT INTO source_documents( \
               id, money_source_id, file_sha256, semantic_document_key, \
               original_filename, mime_type, byte_size, encrypted_locator, file_state \
             ) VALUES ( \
               'document-existing', 'source-dbs', ?1, 'dbs:checking:2026-06', \
               'DBS-June-2026.pdf', 'application/pdf', 2048, \
               'files/document-existing.ccenv', 'available' \
             )",
            ["e".repeat(64)],
        )
        .expect("seed document");
    connection
        .execute(
            "INSERT INTO parse_runs( \
               id, source_document_id, normalization_profile_id, profile_json, status \
             ) VALUES ( \
               'parse-existing', 'document-existing', 'profile-v1', '{}', 'succeeded' \
             )",
            [],
        )
        .expect("seed parse relationship");

    apply_migration_set(&mut connection, &MIGRATIONS[2..]).expect("apply pending identity");

    let not_null: i64 = connection
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('source_documents') \
             WHERE name = 'semantic_document_key'",
            [],
            |row| row.get(0),
        )
        .expect("read semantic column");
    assert_eq!(not_null, 0);
    let semantic_document_key: String = connection
        .query_row(
            "SELECT semantic_document_key FROM source_documents \
             WHERE id = 'document-existing'",
            [],
            |row| row.get(0),
        )
        .expect("preserve identity");
    assert_eq!(semantic_document_key, "dbs:checking:2026-06");
    let source_document_id: String = connection
        .query_row(
            "SELECT source_document_id FROM parse_runs WHERE id = 'parse-existing'",
            [],
            |row| row.get(0),
        )
        .expect("preserve parse relationship");
    assert_eq!(source_document_id, "document-existing");
    let parse_identity: (String, String) = connection
        .query_row(
            "SELECT logical_run_key, input_hash FROM parse_runs WHERE id = 'parse-existing'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("migrate parse identity");
    assert_eq!(parse_identity, ("parse-existing".to_owned(), String::new()));
    assert!(
        connection
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
            .optional()
            .expect("check foreign keys")
            .is_none()
    );
    assert_eq!(
        connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .expect("foreign keys enabled"),
        1
    );
    connection
        .execute(
            "INSERT INTO source_documents( \
               id, money_source_id, file_sha256, \
               original_filename, mime_type, byte_size, encrypted_locator, file_state \
             ) VALUES ( \
               'document-pending', 'source-dbs', ?1, \
               'Pending.pdf', 'application/pdf', 1024, \
               'files/document-pending.ccenv', 'available' \
             )",
            ["p".repeat(64)],
        )
        .expect("insert pending identity");
    let pending_identity: Option<String> = connection
        .query_row(
            "SELECT semantic_document_key FROM source_documents \
             WHERE id = 'document-pending'",
            [],
            |row| row.get(0),
        )
        .expect("read pending identity");
    assert_eq!(pending_identity, None);
    let source_not_null: i64 = connection
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('source_documents') \
             WHERE name = 'money_source_id'",
            [],
            |row| row.get(0),
        )
        .expect("read source column");
    assert_eq!(source_not_null, 0);
    connection
        .execute(
            "INSERT INTO source_documents( \
               id, file_sha256, original_filename, mime_type, byte_size, \
               encrypted_locator, file_state \
             ) VALUES ( \
               'document-unassigned', ?1, 'Pending.csv', 'text/csv', 1024, \
               'files/document-unassigned.ccenv', 'available' \
             )",
            ["u".repeat(64)],
        )
        .expect("insert unassigned source");
    let pending_source: Option<String> = connection
        .query_row(
            "SELECT money_source_id FROM source_documents \
             WHERE id = 'document-unassigned'",
            [],
            |row| row.get(0),
        )
        .expect("read pending source");
    assert_eq!(pending_source, None);

    let invalid = [Migration {
        version: 99,
        sql: "INSERT INTO parse_runs( \
                id, source_document_id, normalization_profile_id, logical_run_key, profile_json, input_hash, status \
              ) VALUES ( \
                'parse-invalid', 'missing-document', 'profile-v1', 'parse-invalid', '{}', '', 'failed' \
              )",
        foreign_keys_off: true,
    }];
    assert!(apply_migration_set(&mut connection, &invalid).is_err());
    assert_eq!(
        connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .expect("foreign keys restored after failure"),
        1
    );
    let invalid_rows: i64 = connection
        .query_row(
            "SELECT count(*) FROM parse_runs WHERE id = 'parse-invalid'",
            [],
            |row| row.get(0),
        )
        .expect("invalid migration rolled back");
    assert_eq!(invalid_rows, 0);
}

#[test]
fn routes_trusted_classification_to_one_source_and_reuses_the_account() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.csv");
    fs::write(&source_path, b"synthetic statement").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &SourceDocumentImport {
                audit_actor: "user",
                audit_id: "audit-import",
                audit_policy_version: "manual-import-v1",
                audit_reason: "manual_import",
                document_id: "document-unassigned",
                mime_type: "text/csv",
                original_filename: "statement.csv",
                source_path: &source_path,
            },
            None,
        )
        .expect("capture unassigned source");
    let accounts = [TrustedAccountCandidate {
        account_id: "account-candidate",
        account_type: "deposit_account",
        currency: Some("SGD"),
        display_name: "Synthetic checking",
        masked_identifier: Some("••001"),
        provider_account_id: Some("checking-001"),
    }];
    let routed = store
        .apply_trusted_classification(&TrustedDocumentClassification {
            accounts: &accounts,
            audit_id: "audit-classify",
            document_id: "document-unassigned",
            document_type: Some("account_statement"),
            provider_key: "dbs",
            semantic_document_key: "dbs:checking:2026-07",
            statement_period_from: Some("2026-07-01"),
            statement_period_to: Some("2026-07-31"),
        })
        .expect("route classification");
    assert_eq!(routed.status, SourceDocumentRoutingStatus::Routed);
    assert_eq!(routed.money_source_id.as_deref(), Some("source-dbs"));
    assert_eq!(routed.account_ids, vec!["account-candidate"]);
    assert!(
        store
            .list_unassigned_documents()
            .expect("list pending")
            .is_empty()
    );
    let documents = store.list_documents("source-dbs").expect("list routed");
    assert_eq!(documents.len(), 1);
    assert_eq!(
        documents[0].semantic_document_key.as_deref(),
        Some("dbs:checking:2026-07")
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT instrument_type, symbol, currency, display_name \
                 FROM instruments WHERE id = 'instrument-fiat-SGD'",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .expect("create deterministic SGD fiat instrument"),
        (
            "fiat_currency".to_owned(),
            "SGD".to_owned(),
            Some("SGD".to_owned()),
            "SGD".to_owned(),
        )
    );

    let repeated_accounts = [TrustedAccountCandidate {
        account_id: "account-ignored",
        account_type: "deposit_account",
        currency: Some("SGD"),
        display_name: "Synthetic checking",
        masked_identifier: Some("••001"),
        provider_account_id: Some("checking-001"),
    }];
    let repeated = store
        .apply_trusted_classification(&TrustedDocumentClassification {
            accounts: &repeated_accounts,
            audit_id: "audit-classify-repeat",
            document_id: "document-unassigned",
            document_type: Some("account_statement"),
            provider_key: "dbs",
            semantic_document_key: "dbs:checking:2026-07",
            statement_period_from: Some("2026-07-01"),
            statement_period_to: Some("2026-07-31"),
        })
        .expect("repeat classification");
    assert_eq!(repeated.account_ids, vec!["account-candidate"]);
    let fiat_instrument_count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM instruments \
             WHERE instrument_type = 'fiat_currency' AND symbol = 'SGD' AND currency = 'SGD'",
            [],
            |row| row.get(0),
        )
        .expect("count deterministic fiat instruments after retry");
    assert_eq!(fiat_instrument_count, 1);
}

#[test]
fn reuses_a_legacy_fiat_instrument_for_matching_currency() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    store
        .connection
        .execute(
            "INSERT INTO source_documents( \
               id, file_sha256, original_filename, mime_type, byte_size, encrypted_locator, file_state \
             ) VALUES ('document-legacy', ?1, 'legacy.pdf', 'application/pdf', 1, \
                       'files/document-legacy.ccenv', 'available')",
            ["l".repeat(64)],
        )
        .expect("seed unassigned legacy document");
    store
        .connection
        .execute(
            "INSERT INTO instruments(id, instrument_type, symbol, currency, display_name) \
             VALUES ('instrument-sgd', 'fiat_currency', 'S$', 'SGD', 'Singapore Dollar')",
            [],
        )
        .expect("seed legacy SGD instrument with display symbol");
    let accounts = [TrustedAccountCandidate {
        account_id: "account-legacy",
        account_type: "deposit_account",
        currency: Some("SGD"),
        display_name: "Legacy checking",
        masked_identifier: Some("••001"),
        provider_account_id: Some("checking-legacy"),
    }];

    let routed = store
        .apply_trusted_classification(&TrustedDocumentClassification {
            accounts: &accounts,
            audit_id: "audit-classify-legacy",
            document_id: "document-legacy",
            document_type: Some("account_statement"),
            provider_key: "dbs",
            semantic_document_key: "dbs:legacy:2026-07",
            statement_period_from: Some("2026-07-01"),
            statement_period_to: Some("2026-07-31"),
        })
        .expect("route through legacy currency instrument");

    assert_eq!(routed.status, SourceDocumentRoutingStatus::Routed);
    let instruments = store
        .connection
        .prepare(
            "SELECT id FROM instruments \
             WHERE instrument_type = 'fiat_currency' AND currency = 'SGD' \
             ORDER BY id",
        )
        .expect("prepare fiat instrument query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query fiat instruments")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect fiat instruments");
    assert_eq!(instruments, vec!["instrument-sgd"]);
}

#[test]
fn rejects_multiple_legacy_fiat_instruments_for_one_currency() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    store
        .connection
        .execute_batch(
            "INSERT INTO instruments(id, instrument_type, symbol, currency, display_name) \
             VALUES ('instrument-sgd-one', 'fiat_currency', 'S$', 'SGD', 'Singapore Dollar'); \
             INSERT INTO instruments(id, instrument_type, symbol, currency, display_name) \
             VALUES ('instrument-sgd-two', 'fiat_currency', 'SG$', 'SGD', 'Singapore Dollar');",
        )
        .expect("seed duplicate legacy SGD instruments");
    let accounts = [TrustedAccountCandidate {
        account_id: "account-duplicate",
        account_type: "deposit_account",
        currency: Some("SGD"),
        display_name: "Duplicate checking",
        masked_identifier: Some("••001"),
        provider_account_id: Some("checking-duplicate"),
    }];
    let transaction = store
        .connection
        .transaction()
        .expect("start fiat instrument transaction");

    let error = ensure_fiat_currency_instruments(&transaction, &accounts)
        .expect_err("reject duplicate fiat instruments for one currency");

    assert!(
        error
            .to_string()
            .contains("multiple fiat currency instruments exist")
    );
}

#[test]
fn rejects_a_conflicting_deterministic_fiat_instrument() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    store
        .connection
        .execute(
            "INSERT INTO instruments(id, instrument_type, symbol, currency, display_name) \
             VALUES ('instrument-fiat-SGD', 'stock', 'SGD', 'SGD', 'Conflicting SGD')",
            [],
        )
        .expect("seed conflicting deterministic instrument");
    let accounts = [TrustedAccountCandidate {
        account_id: "account-conflict",
        account_type: "deposit_account",
        currency: Some("SGD"),
        display_name: "Conflicting checking",
        masked_identifier: Some("••001"),
        provider_account_id: Some("checking-conflict"),
    }];
    let transaction = store
        .connection
        .transaction()
        .expect("start fiat instrument transaction");

    let error = ensure_fiat_currency_instruments(&transaction, &accounts)
        .expect_err("reject conflicting deterministic instrument");

    assert!(
        error
            .to_string()
            .contains("fiat currency instrument conflicts with its deterministic identity")
    );
}

#[test]
fn leaves_ambiguous_source_or_account_classification_unassigned() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF synthetic statement").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .connection
        .execute(
            "INSERT INTO money_sources(id, provider_key, display_name, source_type) \
             VALUES ('source-dbs-second', 'dbs', 'DBS second', 'bank')",
            [],
        )
        .expect("seed ambiguous source");
    store
        .register_import(
            &SourceDocumentImport {
                audit_actor: "user",
                audit_id: "audit-import",
                audit_policy_version: "manual-import-v1",
                audit_reason: "manual_import",
                document_id: "document-unassigned",
                mime_type: "application/pdf",
                original_filename: "statement.pdf",
                source_path: &source_path,
            },
            None,
        )
        .expect("capture unassigned source");
    let accounts = [TrustedAccountCandidate {
        account_id: "account-candidate",
        account_type: "deposit_account",
        currency: Some("SGD"),
        display_name: "Synthetic checking",
        masked_identifier: None,
        provider_account_id: Some("checking-001"),
    }];
    let ambiguous = store
        .apply_trusted_classification(&TrustedDocumentClassification {
            accounts: &accounts,
            audit_id: "audit-classify",
            document_id: "document-unassigned",
            document_type: Some("account_statement"),
            provider_key: "dbs",
            semantic_document_key: "dbs:checking:2026-07",
            statement_period_from: Some("2026-07-01"),
            statement_period_to: Some("2026-07-31"),
        })
        .expect("classify ambiguous source");
    assert_eq!(
        ambiguous.status,
        SourceDocumentRoutingStatus::NeedsAttention
    );
    assert_eq!(ambiguous.reason, Some("money_source_ambiguous"));
    assert_eq!(
        store
            .list_unassigned_documents()
            .expect("list pending")
            .len(),
        1
    );
    let account_count: i64 = store
        .connection
        .query_row("SELECT count(*) FROM accounts", [], |row| row.get(0))
        .expect("count accounts");
    assert_eq!(account_count, 0);
}

#[test]
fn imports_deduplicates_groups_and_restores_source_documents() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let first_path = root.path().join("first.pdf");
    fs::write(&first_path, b"%PDF synthetic first statement").expect("write first fixture");
    let mut store = open_store(root.path());

    let first = store
        .register_import(&import(&first_path, "document-first", "audit-first"), None)
        .expect("first import");
    assert_eq!(first.status, SourceDocumentImportStatus::Imported);
    let duplicate = store
        .register_import(
            &import(&first_path, "document-ignored", "audit-duplicate"),
            None,
        )
        .expect("duplicate import");
    assert_eq!(duplicate.status, SourceDocumentImportStatus::AlreadyPresent);

    let second_path = root.path().join("second.pdf");
    fs::write(&second_path, b"%PDF synthetic rescanned statement").expect("write second fixture");
    let second = store
        .register_import(
            &import(&second_path, "document-second", "audit-second"),
            None,
        )
        .expect("second import");
    assert_eq!(second.status, SourceDocumentImportStatus::Imported);

    let first_sha256 = FileVault::prepare(&first_path)
        .expect("prepare first source")
        .file_sha256()
        .to_owned();
    let first_document = find_exact_document(&store.connection, &first_sha256)
        .expect("find document")
        .expect("existing document");
    let encrypted_path = root.path().join(
        first_document
            .encrypted_locator
            .as_deref()
            .expect("available locator"),
    );
    store
        .connection
        .execute(
            "INSERT INTO parse_runs( \
               id, source_document_id, normalization_profile_id, logical_run_key, profile_json, input_hash, status \
             ) VALUES ('parse-first', ?1, 'profile-v1', 'parse-first', '{}', '', 'succeeded')",
            [&first_document.document_id],
        )
        .expect("seed retained parse relationship");
    store
        .connection
        .execute(
            "INSERT INTO external_records( \
               id, parse_run_id, source_document_id, stable_record_key, version, \
               status, record_type, raw_json, validation_json \
             ) VALUES \
               ('record-staged', 'parse-first', ?1, 'record-staged', 1, \
                'staged', 'transaction', '{}', '{}'), \
               ('record-review', 'parse-first', ?1, 'record-review', 1, \
                'review', 'transaction', '{}', '{}'), \
               ('record-committed', 'parse-first', ?1, 'record-committed', 1, \
                'committed', 'transaction', '{}', '{}')",
            [&first_document.document_id],
        )
        .expect("seed linked records");

    store
        .delete_source_document(&first_document.document_id, "audit-delete")
        .expect("delete encrypted source");
    assert!(!encrypted_path.exists());
    let deleted = find_document_by_id(&store.connection, &first_document.document_id)
        .expect("find deleted document")
        .expect("deleted document remains");
    assert_eq!(deleted.file_state, "deleted");
    assert!(deleted.encrypted_locator.is_none());
    let retained_parses: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM parse_runs WHERE source_document_id = ?1",
            [&first_document.document_id],
            |row| row.get(0),
        )
        .expect("count retained parse relationships");
    assert_eq!(retained_parses, 1);
    let record_states = store
        .connection
        .prepare(
            "SELECT id, status FROM external_records \
             WHERE source_document_id = ?1 ORDER BY id",
        )
        .expect("prepare linked record query")
        .query_map([&first_document.document_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("query linked records")
        .collect::<Result<Vec<_>, _>>()
        .expect("read linked records");
    assert_eq!(
        record_states,
        vec![
            ("record-committed".to_owned(), "committed".to_owned()),
            ("record-review".to_owned(), "review".to_owned()),
            ("record-staged".to_owned(), "review".to_owned()),
        ]
    );
    let deletion_review_items: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM review_items \
             WHERE reason_code = 'source_file_deleted' AND status = 'open'",
            [],
            |row| row.get(0),
        )
        .expect("count source deletion review items");
    assert_eq!(deletion_review_items, 2);

    assert!(
        store
            .delete_source_document(&first_document.document_id, "audit-delete-retry")
            .is_err()
    );
    let deletion_audits: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM audit_log \
             WHERE entity_id = ?1 AND action = 'source_file_deletion_decided'",
            [&first_document.document_id],
            |row| row.get(0),
        )
        .expect("count deletion decisions after retry");
    assert_eq!(deletion_audits, 1);

    let confirmation = store
        .register_import(
            &import(&first_path, "document-ignored-restored", "audit-restored"),
            None,
        )
        .expect("request restore confirmation");
    assert_eq!(confirmation.document_id, first_document.document_id);
    assert_eq!(
        confirmation.status,
        SourceDocumentImportStatus::RestoreConfirmationRequired
    );
    assert!(!encrypted_path.exists());

    fs::write(&first_path, b"%PDF changed after restore confirmation")
        .expect("change selected source");
    assert!(
        store
            .register_import(
                &import(&first_path, "document-changed", "audit-changed"),
                Some(&first_document.document_id),
            )
            .is_err()
    );
    assert_eq!(store.list_unassigned_documents().expect("list").len(), 2);
    fs::write(&first_path, b"%PDF synthetic first statement")
        .expect("restore selected source fixture");

    let restored = store
        .register_import(
            &import(&first_path, "document-ignored-restored", "audit-restored"),
            Some(&first_document.document_id),
        )
        .expect("restore exact source");
    assert_eq!(restored.document_id, first_document.document_id);
    assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
    assert_eq!(store.list_unassigned_documents().expect("list").len(), 2);
}

#[test]
fn deletion_recovery_removes_a_blob_left_after_the_tombstone_commit() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF deletion recovery statement").expect("write fixture");
    let mut store = open_store(root.path());
    let imported = store
        .register_import(
            &import(&source_path, "document-delete", "audit-import"),
            None,
        )
        .expect("import source");
    let document = find_document_by_id(&store.connection, &imported.document_id)
        .expect("find document")
        .expect("imported document");
    let encrypted_path = root.path().join(
        document
            .encrypted_locator
            .as_deref()
            .expect("available locator"),
    );

    persist_source_deletion(&mut store.connection, &document, "audit-delete")
        .expect("commit deletion decision");
    assert!(
        encrypted_path.exists(),
        "simulate crash before blob removal"
    );
    drop(store);

    let reopened = open_store(root.path());
    assert!(!encrypted_path.exists());
    let tombstone = find_document_by_id(&reopened.connection, &imported.document_id)
        .expect("find tombstone")
        .expect("document row retained");
    assert_eq!(tombstone.file_state, "deleted");
    let deletion_audits: i64 = reopened
        .connection
        .query_row(
            "SELECT count(*) FROM audit_log \
             WHERE entity_id = ?1 AND action = 'source_file_deletion_decided'",
            [&imported.document_id],
            |row| row.get(0),
        )
        .expect("count deletion audit");
    assert_eq!(deletion_audits, 1);
}

#[test]
fn missing_storage_is_not_recorded_as_a_user_deletion() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF missing before deletion").expect("write fixture");
    let mut store = open_store(root.path());
    let imported = store
        .register_import(
            &import(&source_path, "document-missing-delete", "audit-import"),
            None,
        )
        .expect("import source");
    let locator = store.list_unassigned_documents().expect("list")[0]
        .encrypted_locator
        .clone()
        .expect("available locator");
    store
        .files
        .remove(&locator)
        .expect("remove encrypted fixture");

    assert!(
        store
            .delete_source_document(&imported.document_id, "audit-delete")
            .is_err()
    );
    let document = find_document_by_id(&store.connection, &imported.document_id)
        .expect("find document")
        .expect("document row retained");
    assert_eq!(document.file_state, "missing");
    let deletion_audits: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM audit_log \
             WHERE entity_id = ?1 AND action = 'source_file_deletion_decided'",
            [&imported.document_id],
            |row| row.get(0),
        )
        .expect("count deletion audit");
    assert_eq!(deletion_audits, 0);
}

#[test]
fn defers_unreferenced_file_cleanup_until_the_next_vault_open() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF rollback statement").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .connection
        .execute(
            "INSERT INTO audit_log( \
               id, entity_type, entity_id, action, actor, reason, policy_version \
             ) VALUES ('audit-conflict', 'test', 'test', 'seed', 'test', 'test', 'test')",
            [],
        )
        .expect("seed audit conflict");

    assert!(
        store
            .register_import(
                &import(&source_path, "document-rollback", "audit-conflict"),
                None,
            )
            .is_err()
    );
    let rows: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM source_documents WHERE id = 'document-rollback'",
            [],
            |row| row.get(0),
        )
        .expect("count rolled-back documents");
    assert_eq!(rows, 0);
    let files = root.path().join("files");
    assert_eq!(fs::read_dir(&files).expect("files directory").count(), 1);
    drop(store);
    let reopened = open_store(root.path());
    assert_eq!(fs::read_dir(files).expect("files directory").count(), 0);
    drop(reopened);
}

#[test]
fn marks_an_absent_registered_file_missing_and_restores_it_on_reimport() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF missing statement").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import(&source_path, "document-missing", "audit-import"),
            None,
        )
        .expect("import source");
    let locator = store.list_unassigned_documents().expect("list")[0]
        .encrypted_locator
        .clone()
        .expect("available locator");
    store
        .files
        .remove(&locator)
        .expect("remove file outside database");
    drop(store);

    let mut reopened = open_store(root.path());
    assert_eq!(
        reopened.list_unassigned_documents().expect("list")[0].file_state,
        "missing"
    );
    let restored = reopened
        .register_import(
            &import(&source_path, "document-ignored", "audit-restore"),
            None,
        )
        .expect("restore missing source");
    assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
}

#[test]
fn marks_a_tampered_registered_file_missing_on_reopen() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF tampered statement").expect("write fixture");
    let mut store = open_store(root.path());
    store
        .register_import(
            &import(&source_path, "document-tampered", "audit-import"),
            None,
        )
        .expect("import source");
    let locator = store.list_unassigned_documents().expect("list")[0]
        .encrypted_locator
        .clone()
        .expect("available locator");
    let encrypted_path = root.path().join(locator);
    let mut envelope = fs::read(&encrypted_path).expect("read encrypted fixture");
    *envelope.last_mut().expect("non-empty envelope") ^= 1;
    fs::write(encrypted_path, envelope).expect("tamper encrypted fixture");
    drop(store);

    let reopened = open_store(root.path());
    assert_eq!(
        reopened.list_unassigned_documents().expect("list")[0].file_state,
        "missing"
    );
}

#[test]
fn restores_a_valid_envelope_found_at_the_wrong_document_locator() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let first_path = root.path().join("first.pdf");
    let second_path = root.path().join("second.pdf");
    fs::write(&first_path, b"%PDF first statement").expect("write first fixture");
    fs::write(&second_path, b"%PDF second statement").expect("write second fixture");
    let mut store = open_store(root.path());
    store
        .register_import(&import(&first_path, "document-first", "audit-first"), None)
        .expect("import first source");
    store
        .register_import(
            &import(&second_path, "document-second", "audit-second"),
            None,
        )
        .expect("import second source");

    let first_sha256 = FileVault::prepare(&first_path)
        .expect("prepare first source")
        .file_sha256()
        .to_owned();
    let second_sha256 = FileVault::prepare(&second_path)
        .expect("prepare second source")
        .file_sha256()
        .to_owned();
    let first = find_exact_document(&store.connection, &first_sha256)
        .expect("find first document")
        .expect("first document");
    let second = find_exact_document(&store.connection, &second_sha256)
        .expect("find second document")
        .expect("second document");
    let first_envelope = fs::read(
        root.path()
            .join(first.encrypted_locator.expect("first locator")),
    )
    .expect("read first envelope");
    fs::write(
        root.path()
            .join(second.encrypted_locator.expect("second locator")),
        first_envelope,
    )
    .expect("misplace valid envelope");
    drop(store);

    let mut store = open_store(root.path());
    let documents = store.list_unassigned_documents().expect("list");
    let second_document = documents
        .iter()
        .find(|document| document.document_id == "document-second")
        .expect("second document view");
    assert_eq!(second_document.file_state, "missing");

    let restored = store
        .register_import(
            &import(&second_path, "document-ignored", "audit-restored"),
            None,
        )
        .expect("restore second source");
    assert_eq!(restored.document_id, "document-second");
    assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
}

#[test]
fn rejects_the_wrong_database_key_and_cleans_an_unregistered_envelope_on_open() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let orphan_path = root.path().join("orphan.pdf");
    fs::write(&orphan_path, b"unregistered evidence").expect("write orphan source");
    let store = open_store(root.path());
    let orphan = store
        .files
        .store(&KEY, &orphan_path)
        .expect("store orphaned envelope");
    let encrypted_path = root.path().join(&orphan.encrypted_locator);
    assert!(encrypted_path.exists());
    drop(store);

    let reopened = open_store(root.path());
    assert!(!encrypted_path.exists());
    drop(reopened);
    assert!(ManualImportStore::open(root.path(), Zeroizing::new([0x92; KEY_LEN])).is_err());
    let database_bytes =
        fs::read(root.path().join("finance.sqlite")).expect("read encrypted database");
    assert!(
        !database_bytes
            .windows("source-dbs".len())
            .any(|part| part == b"source-dbs")
    );
}

#[test]
fn persists_only_statement_password_reference_state() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let store = open_store(root.path());

    assert_eq!(
        store
            .statement_password_state("source-dbs")
            .expect("initial statement password state"),
        None
    );

    store
        .begin_statement_password_save("source-dbs", "money-source:source-dbs")
        .expect("begin save");
    assert_eq!(
        store
            .statement_password_state("source-dbs")
            .expect("pending save state"),
        Some(StatementPasswordState {
            money_source_id: "source-dbs".to_owned(),
            secret_storage_key: "money-source:source-dbs".to_owned(),
            status: StatementPasswordStatus::PendingSave,
        })
    );
    assert_eq!(
        store
            .pending_statement_password_states()
            .expect("pending states")
            .len(),
        1
    );

    store
        .mark_statement_password_saved("source-dbs")
        .expect("finish save");
    assert_eq!(
        store
            .statement_password_state("source-dbs")
            .expect("saved state")
            .expect("saved reference")
            .status,
        StatementPasswordStatus::Saved
    );

    store
        .begin_statement_password_delete("source-dbs")
        .expect("begin delete");
    assert_eq!(
        store
            .statement_password_state("source-dbs")
            .expect("pending delete state")
            .expect("pending delete reference")
            .status,
        StatementPasswordStatus::PendingDelete
    );
    store
        .remove_statement_password_ref("source-dbs", "pending_delete")
        .expect("finish delete");
    assert_eq!(
        store
            .statement_password_state("source-dbs")
            .expect("cleared statement password state"),
        None
    );
}

fn seed_review_repayment(store: &mut ManualImportStore, card_first: bool) {
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
               ('account-dbs-card', 'source-dbs', 'dbs', 'card-1', 'credit_card', \
                'DBS Visa', 'SGD', 'confirmed')",
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
    let documents = if card_first {
        [
            (
                "document-dbs-july",
                "source-dbs",
                "d".repeat(64),
                "dbs:card:2026-07",
            ),
            (
                "document-hsbc-june",
                "source-hsbc",
                "h".repeat(64),
                "hsbc:cash:2026-06",
            ),
        ]
    } else {
        [
            (
                "document-hsbc-june",
                "source-hsbc",
                "h".repeat(64),
                "hsbc:cash:2026-06",
            ),
            (
                "document-dbs-july",
                "source-dbs",
                "d".repeat(64),
                "dbs:card:2026-07",
            ),
        ]
    };
    for (id, source_id, sha, semantic_key) in documents {
        store
            .connection
            .execute(
                "INSERT INTO source_documents( \
                   id, money_source_id, file_sha256, semantic_document_key, \
                   original_filename, mime_type, byte_size, file_state \
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'text/csv', 1, 'missing')",
                params![id, source_id, sha, semantic_key, format!("{id}.csv")],
            )
            .expect("seed statement document");
        store
            .connection
            .execute(
                "INSERT INTO parse_runs( \
                   id, source_document_id, normalization_profile_id, logical_run_key, profile_json, input_hash, status \
                 ) VALUES (?1, ?2, 'synthetic-review-v1', ?1, '{}', '', 'validated')",
                params![format!("parse-{id}"), id],
            )
            .expect("seed parse run");
    }
    let records = if card_first {
        [
            (
                "record-dbs-card",
                "parse-document-dbs-july",
                "document-dbs-july",
                "account-dbs-card",
                "dbs-card-payment",
                "2026-07-01",
            ),
            (
                "record-hsbc-cash",
                "parse-document-hsbc-june",
                "document-hsbc-june",
                "account-hsbc-cash",
                "hsbc-card-payment",
                "2026-06-30",
            ),
        ]
    } else {
        [
            (
                "record-hsbc-cash",
                "parse-document-hsbc-june",
                "document-hsbc-june",
                "account-hsbc-cash",
                "hsbc-card-payment",
                "2026-06-30",
            ),
            (
                "record-dbs-card",
                "parse-document-dbs-july",
                "document-dbs-july",
                "account-dbs-card",
                "dbs-card-payment",
                "2026-07-01",
            ),
        ]
    };
    for (id, parse_run_id, document_id, account_id, stable_key, posted_on) in records {
        store
            .connection
            .execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                   status, record_type, event_type, posted_on, amount_value, currency, \
                   account_balance_delta, raw_json, validation_json \
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 1, 'review', 'transaction', \
                           'credit_card_repayment', ?6, '750.00', 'SGD', '-750.00', \
                           '{\"private\":\"must-not-leak\"}', '{\"internal\":true}')",
                params![
                    id,
                    parse_run_id,
                    document_id,
                    account_id,
                    stable_key,
                    posted_on
                ],
            )
            .expect("seed review record");
        store
            .connection
            .execute(
                "INSERT INTO review_items(id, external_record_id, reason_code, status) \
                 VALUES (?1, ?2, 'possible_card_repayment', 'open')",
                params![format!("review-{id}"), id],
            )
            .expect("seed review item");
    }
}

fn prepared_repayment() -> CorePreparedReviewEvent {
    CorePreparedReviewEvent {
        event_class: "posting".to_owned(),
        event_date: "2026-06-30".to_owned(),
        event_type: "credit_card_repayment".to_owned(),
        source_record_ids: vec!["record-dbs-card".to_owned(), "record-hsbc-cash".to_owned()],
        legs: vec![
            CoreReviewLeg {
                account_id: "account-dbs-card".to_owned(),
                instrument_id: "instrument-sgd".to_owned(),
                currency: "SGD".to_owned(),
                amount_value: "-750.00".to_owned(),
            },
            CoreReviewLeg {
                account_id: "account-hsbc-cash".to_owned(),
                instrument_id: "instrument-sgd".to_owned(),
                currency: "SGD".to_owned(),
                amount_value: "-750.00".to_owned(),
            },
        ],
        spending: false,
    }
}

#[test]
fn limits_relationship_candidates_within_the_supported_window_before_capping_results() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);

    for index in 0..101 {
        let id = format!("record-old-{index:03}");
        store
            .connection
            .execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                   status, record_type, event_type, posted_on, amount_value, currency, \
                   account_balance_delta, raw_json, validation_json \
                 ) VALUES (?1, 'parse-document-dbs-july', 'document-dbs-july', \
                           'account-dbs-card', ?2, 1, 'review', 'transaction', \
                           'credit_card_repayment', '2025-01-01', '750.00', 'SGD', '-750.00', \
                           '{}', '{}')",
                params![id, format!("old-repayment-{index}")],
            )
            .expect("seed older relationship candidate");
    }

    let candidate_input = store
        .relationship_candidate_input("review-record-hsbc-cash", 1)
        .expect("load candidate input")
        .expect("current review record");
    assert_eq!(
        candidate_input
            .candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<Vec<_>>(),
        vec!["record-dbs-card"]
    );
}

#[test]
fn qualifies_relationship_candidates_before_capping_results() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);

    for index in 0..101 {
        let id = format!("record-mismatch-{index:03}");
        store
            .connection
            .execute(
                "INSERT INTO external_records( \
                   id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
                   status, record_type, event_type, posted_on, amount_value, currency, \
                   account_balance_delta, raw_json, validation_json \
                 ) VALUES (?1, 'parse-document-dbs-july', 'document-dbs-july', \
                           'account-dbs-card', ?2, 1, 'review', 'transaction', \
                           'credit_card_repayment', '2026-06-30', '1.00', 'SGD', '-1.00', \
                           '{}', '{}')",
                params![id, format!("mismatched-repayment-{index}")],
            )
            .expect("seed magnitude-mismatched candidate");
    }
    store
        .connection
        .execute(
            "INSERT INTO external_records( \
               id, parse_run_id, source_document_id, account_id, stable_record_key, version, \
               status, record_type, event_type, posted_on, amount_value, currency, \
               account_balance_delta, raw_json, validation_json \
             ) VALUES ('record-dbs-card-second', 'parse-document-dbs-july', \
                       'document-dbs-july', 'account-dbs-card', 'dbs-card-repayment-second', \
                       1, 'review', 'transaction', 'credit_card_repayment', '2026-07-02', \
                       '750.00', 'SGD', '-750.0', '{}', '{}')",
            [],
        )
        .expect("seed second qualified candidate");

    let candidate_input = store
        .relationship_candidate_input("review-record-hsbc-cash", 1)
        .expect("load candidate input")
        .expect("current review record");
    assert_eq!(
        candidate_input
            .candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<Vec<_>>(),
        vec!["record-dbs-card", "record-dbs-card-second"]
    );
}

#[test]
fn advertises_undo_only_for_supported_review_event_types() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let store = open_store(root.path());
    for (id, event_type) in [
        ("event-repayment", "credit_card_repayment"),
        ("event-transfer", "same_currency_transfer"),
        ("event-purchase", "purchase"),
        ("event-other", "manual_adjustment"),
    ] {
        store
            .connection
            .execute(
                "INSERT INTO ledger_events( \
                   id, event_type, event_class, event_date, status, commit_idempotency_key \
                 ) VALUES (?1, ?2, 'posting', '2026-07-01', 'committed', ?3)",
                params![id, event_type, format!("activity-{id}")],
            )
            .expect("seed committed activity");
    }

    let activity = store.list_recent_activity().expect("list activity");
    assert!(
        activity
            .iter()
            .find(|item| item.event_id == "event-repayment")
            .expect("repayment activity")
            .can_undo
    );
    assert!(
        activity
            .iter()
            .find(|item| item.event_id == "event-transfer")
            .expect("transfer activity")
            .can_undo
    );
    assert!(
        !activity
            .iter()
            .find(|item| item.event_id == "event-purchase")
            .expect("purchase activity")
            .can_undo
    );
    assert!(
        !activity
            .iter()
            .find(|item| item.event_id == "event-other")
            .expect("other activity")
            .can_undo
    );
}

#[test]
fn keeps_an_accepted_relationship_in_review_until_both_sides_are_selected() {
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
        .expect("accept exact repayment relationship");
    let job = store
        .enqueue_commit_review_batch(&["review-record-hsbc-cash".to_owned()])
        .expect("enqueue one selected side");
    let claimed = store
        .claim_review_batch(&job.job_id, "test-worker")
        .expect("claim batch")
        .expect("queued job");

    let (groups, outcomes) = store
        .prepare_commit_review_groups(&claimed)
        .expect("prepare selected review items");

    assert!(groups.is_empty());
    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0],
        ReviewBatchGroupOutcome {
            reason: Some("relationship_not_selected".to_owned()),
            record_ids: vec!["record-hsbc-cash".to_owned()],
            status: ReviewBatchGroupStatus::StillNeedsReview,
        }
    );
}

#[test]
fn blocks_review_commit_until_relationship_accounts_are_confirmed() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    store
        .connection
        .execute(
            "UPDATE accounts SET status = 'candidate' WHERE id = 'account-dbs-card'",
            [],
        )
        .expect("make card account a candidate");
    store
        .accept_review_relationship(
            "review-record-hsbc-cash",
            1,
            "record-dbs-card",
            1,
            &prepared_repayment(),
        )
        .expect("accept exact repayment relationship");
    let job = store
        .enqueue_commit_review_batch(&[
            "review-record-hsbc-cash".to_owned(),
            "review-record-dbs-card".to_owned(),
        ])
        .expect("enqueue selected relationship");
    let claimed = store
        .claim_review_batch(&job.job_id, "test-worker")
        .expect("claim batch")
        .expect("queued job");
    let (groups, outcomes) = store
        .prepare_commit_review_groups(&claimed)
        .expect("prepare selected relationship");
    assert!(outcomes.is_empty());
    assert_eq!(groups.len(), 1);

    let blocked = store
        .commit_prepared_review_group(&claimed, &groups[0], &prepared_repayment())
        .expect("block candidate account commit");

    assert_eq!(blocked.status, ReviewBatchGroupStatus::StillNeedsReview);
    assert_eq!(
        blocked.reason.as_deref(),
        Some("account_confirmation_required")
    );
    let event_count: i64 = store
        .connection
        .query_row("SELECT count(*) FROM ledger_events", [], |row| row.get(0))
        .expect("count ledger events");
    assert_eq!(event_count, 0);

    assert_eq!(
        store
            .confirm_candidate_accounts(
                "source-dbs",
                &["account-dbs-card".to_owned()],
                "audit-confirm-card",
            )
            .expect("confirm candidate account"),
        AccountConfirmationOutcome {
            status: AccountConfirmationStatus::Confirmed,
        }
    );
    let committed = store
        .commit_prepared_review_group(&claimed, &groups[0], &prepared_repayment())
        .expect("commit after account confirmation");
    assert_eq!(committed.status, ReviewBatchGroupStatus::Committed);
    let committed_event_count: i64 = store
        .connection
        .query_row("SELECT count(*) FROM ledger_events", [], |row| row.get(0))
        .expect("count committed ledger events");
    assert_eq!(committed_event_count, 1);
}

#[test]
fn persists_cross_month_repayment_review_commit_undo_and_restart_recovery() {
    for card_first in [false, true] {
        let root = tempfile::tempdir().expect("temporary Vault");
        let mut store = open_store(root.path());
        seed_review_repayment(&mut store, card_first);

        let candidate_input = store
            .relationship_candidate_input("review-record-hsbc-cash", 1)
            .expect("load candidate input")
            .expect("current review record");
        assert_eq!(candidate_input.record.posted_on, "2026-06-30");
        assert_eq!(candidate_input.event_type, "credit_card_repayment");
        assert_eq!(
            candidate_input
                .candidates
                .iter()
                .map(|candidate| candidate.id.as_str())
                .collect::<Vec<_>>(),
            vec!["record-dbs-card"]
        );
        let accepted = store
            .accept_review_relationship(
                "review-record-hsbc-cash",
                1,
                "record-dbs-card",
                1,
                &prepared_repayment(),
            )
            .expect("accept exact repayment relationship");
        assert_eq!(accepted.status, ReviewMutationStatus::RelationshipAccepted);
        let detail = store
            .review_item_detail("review-record-hsbc-cash")
            .expect("read review detail")
            .expect("detail remains available before commit");
        let serialized = serde_json::to_string(&detail).expect("serialize safe detail");
        assert!(!serialized.contains("rawJson"));
        assert!(!serialized.contains("validationJson"));
        assert!(!serialized.contains("private"));
        assert!(!serialized.contains("locator"));

        let job = store
            .enqueue_commit_review_batch(&[
                "review-record-hsbc-cash".to_owned(),
                "review-record-dbs-card".to_owned(),
            ])
            .expect("enqueue batch");
        let claimed = store
            .claim_review_batch(&job.job_id, "test-worker")
            .expect("claim batch")
            .expect("queued job claims once");
        let (groups, initial_outcomes) = store
            .prepare_commit_review_groups(&claimed)
            .expect("group accepted relationship");
        assert!(initial_outcomes.is_empty());
        assert_eq!(groups.len(), 1);
        let committed = store
            .commit_prepared_review_group(&claimed, &groups[0], &prepared_repayment())
            .expect("commit prepared repayment");
        assert_eq!(committed.status, ReviewBatchGroupStatus::Committed);
        store
            .connection
            .execute(
                "UPDATE jobs SET lease_until = datetime('now', '-1 second') WHERE id = ?1",
                [&job.job_id],
            )
            .expect("simulate crash after group commit");
        drop(store);

        let mut reopened = open_store(root.path());
        let recovered = reopened
            .review_job(&job.job_id)
            .expect("read recovered job")
            .expect("job");
        assert_eq!(recovered.status, ReviewJobStatus::Queued);
        assert_eq!(
            reopened
                .queued_review_job_ids()
                .expect("list recovered review jobs"),
            vec![job.job_id.clone()]
        );
        let retry = reopened
            .claim_review_batch(&job.job_id, "retry-worker")
            .expect("claim recovered job")
            .expect("recovered claim");
        let (retry_groups, outcomes) = reopened
            .prepare_commit_review_groups(&retry)
            .expect("rebuild recovered groups");
        assert!(retry_groups.is_empty());
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, ReviewBatchGroupStatus::AlreadyCommitted);
        reopened
            .finish_review_batch(&retry, &outcomes)
            .expect("finish recovered job");
        let event_count: i64 = reopened
            .connection
            .query_row(
                "SELECT count(*) FROM ledger_events WHERE event_type = 'credit_card_repayment'",
                [],
                |row| row.get(0),
            )
            .expect("count committed events");
        let audit_count: i64 = reopened
            .connection
            .query_row(
                "SELECT count(*) FROM audit_log WHERE action = 'review_batch_committed'",
                [],
                |row| row.get(0),
            )
            .expect("count commit audit");
        assert_eq!(event_count, 1);
        assert_eq!(audit_count, 1);
        assert!(
            reopened
                .list_recent_activity()
                .expect("recent activity")
                .iter()
                .all(|item| !item.spending)
        );

        let original_id: String = reopened
            .connection
            .query_row(
                "SELECT id FROM ledger_events WHERE event_type = 'credit_card_repayment'",
                [],
                |row| row.get(0),
            )
            .expect("committed original");
        let original = reopened
            .committed_review_event_for_reversal(&original_id)
            .expect("load original for undo")
            .expect("undo available");
        let reversal = CorePreparedReversalEvent {
            event_type: "credit_card_repayment_reversal".to_owned(),
            event_class: "posting".to_owned(),
            event_date: original.event_date.clone(),
            legs: original
                .legs
                .iter()
                .map(|leg| CoreReviewLeg {
                    account_id: leg.account_id.clone(),
                    instrument_id: leg.instrument_id.clone(),
                    currency: leg.currency.clone(),
                    amount_value: "750.00".to_owned(),
                })
                .collect(),
            spending: false,
        };
        let mut wrong_date = reversal.clone();
        wrong_date.event_date = "2026-07-02".to_owned();
        assert_eq!(
            reopened
                .persist_review_reversal(&original_id, &wrong_date)
                .expect("reject wrong reversal date"),
            None
        );
        let mut wrong_leg = reversal.clone();
        wrong_leg.legs[0].amount_value = "751.00".to_owned();
        assert_eq!(
            reopened
                .persist_review_reversal(&original_id, &wrong_leg)
                .expect("reject wrong reversal leg"),
            None
        );
        assert_eq!(
            reopened
                .persist_review_reversal(&original_id, &reversal)
                .expect("persist undo")
                .expect("undo outcome")
                .status,
            UndoStatus::Undone
        );
        assert_eq!(
            reopened
                .persist_review_reversal(&original_id, &reversal)
                .expect("repeat undo")
                .expect("idempotent undo outcome")
                .status,
            UndoStatus::AlreadyUndone
        );
        assert_eq!(
            reopened
                .connection
                .query_row(
                    "SELECT event_date FROM ledger_events WHERE id = ?1",
                    [&original_id],
                    |row| row.get::<_, String>(0),
                )
                .expect("original stays immutable"),
            "2026-06-30"
        );
    }
}

#[test]
fn rejects_a_stale_review_mutation_without_erasing_raw_or_audit_history() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);

    let edited = store
        .edit_review_record("review-record-hsbc-cash", 1, Some("2026-06-29"), None, None)
        .expect("edit current review record");
    assert_eq!(edited.status, ReviewMutationStatus::Updated);
    assert_eq!(
        store
            .remove_review_record("review-record-hsbc-cash", 1)
            .expect("stale remove is safe"),
        review_conflict("stale_review_item")
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT raw_json FROM external_records WHERE id = 'record-hsbc-cash'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("original raw record remains"),
        "{\"private\":\"must-not-leak\"}"
    );
    let audit_count: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM audit_log WHERE action = 'review_record_edited'",
            [],
            |row| row.get(0),
        )
        .expect("one edit audit");
    assert_eq!(audit_count, 1);
}

#[test]
fn rejects_negative_amount_value_edits_but_allows_signed_balance_delta() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);

    assert_eq!(
        store
            .edit_review_record("review-record-hsbc-cash", 1, None, Some("-750.00"), None)
            .expect("reject negative amount value"),
        review_conflict("invalid_review_edit")
    );
    assert_eq!(
        store
            .edit_review_record("review-record-hsbc-cash", 1, None, None, Some("-750.00"))
            .expect("accept signed balance delta")
            .status,
        ReviewMutationStatus::Updated
    );
}

#[test]
fn records_a_safe_failure_for_a_claimed_review_batch() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    seed_review_repayment(&mut store, false);
    let job = store
        .enqueue_commit_review_batch(&["review-record-hsbc-cash".to_owned()])
        .expect("enqueue batch");
    let claimed = store
        .claim_review_batch(&job.job_id, "test-worker")
        .expect("claim batch")
        .expect("queued job claims once");

    let failed = store
        .fail_review_batch(&claimed, "review_core_failed")
        .expect("persist safe job failure");

    assert_eq!(failed.status, ReviewJobStatus::Failed);
    assert!(failed.outcomes.is_empty());
    let error_json: String = store
        .connection
        .query_row(
            "SELECT error_json FROM jobs WHERE id = ?1",
            [&job.job_id],
            |row| row.get(0),
        )
        .expect("read persisted safe error");
    assert_eq!(error_json, r#"{"errorCode":"review_core_failed"}"#);
}
