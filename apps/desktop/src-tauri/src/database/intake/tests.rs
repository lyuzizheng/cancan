use super::candidates::*;
use super::*;

mod candidates;

const KEY: [u8; KEY_LEN] = [0x71; KEY_LEN];

fn open_store(root: &Path) -> ManualImportStore {
    ManualImportStore::open(root, Zeroizing::new(KEY)).expect("open encrypted Vault")
}

fn insert_document(store: &ManualImportStore, id: &str, hash_byte: char) {
    store
        .connection
        .execute(
            "INSERT INTO source_documents( \
               id, file_sha256, original_filename, mime_type, byte_size, file_state \
             ) VALUES (?1, ?2, 'statement.pdf', 'application/pdf', 1, 'missing')",
            params![id, hash_byte.to_string().repeat(64)],
        )
        .expect("seed source document");
}

fn import_input<'a>(
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

fn local_item<'a>(
    id: &'a str,
    label: &'a str,
    entry_key: &'a str,
    snapshot_version: &'a str,
) -> IntakeBatchItemInput<'a> {
    IntakeBatchItemInput {
        id,
        safe_input_label: label,
        acquisition_input_key: Some(entry_key),
        acquisition_input_version: Some(snapshot_version),
        retry_of_batch_item_id: None,
    }
}

#[test]
fn creates_only_non_empty_valid_batches_as_one_sealed_pending_set() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let items = [
        IntakeBatchItemInput {
            id: "item-a",
            safe_input_label: "first.pdf",
            acquisition_input_key: None,
            acquisition_input_version: None,
            retry_of_batch_item_id: None,
        },
        IntakeBatchItemInput {
            id: "item-b",
            safe_input_label: "second.csv",
            acquisition_input_key: None,
            acquisition_input_version: None,
            retry_of_batch_item_id: None,
        },
    ];

    store
        .create_intake_batch(&IntakeBatchInput {
            id: "batch-valid",
            acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
            items: &items,
        })
        .expect("create sealed batch");

    let batch: (String, String, Option<String>) = store
        .connection
        .query_row(
            "SELECT opened_at, sealed_at, completed_at FROM intake_batches \
             WHERE id = 'batch-valid'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read sealed batch");
    assert_eq!(batch.0, batch.1);
    assert_eq!(batch.2, None);
    let persisted_items = store
        .connection
        .prepare(
            "SELECT id, input_ordinal, capture_outcome, finalized_at \
             FROM intake_batch_items WHERE intake_batch_id = 'batch-valid' \
             ORDER BY input_ordinal",
        )
        .expect("prepare item query")
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .expect("query items")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect items");
    assert_eq!(
        persisted_items,
        vec![
            ("item-a".to_owned(), 0, "pending".to_owned(), None),
            ("item-b".to_owned(), 1, "pending".to_owned(), None),
        ]
    );

    assert!(
        store
            .create_intake_batch(&IntakeBatchInput {
                id: "batch-empty",
                acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
                items: &[],
            })
            .is_err()
    );
    let invalid_items = [
        IntakeBatchItemInput {
            id: "item-valid-before-failure",
            safe_input_label: "valid.pdf",
            acquisition_input_key: None,
            acquisition_input_version: None,
            retry_of_batch_item_id: None,
        },
        IntakeBatchItemInput {
            id: "item-invalid",
            safe_input_label: "hidden\u{202e}fdp.exe",
            acquisition_input_key: None,
            acquisition_input_version: None,
            retry_of_batch_item_id: None,
        },
    ];
    assert!(
        store
            .create_intake_batch(&IntakeBatchInput {
                id: "batch-invalid",
                acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
                items: &invalid_items,
            })
            .is_err()
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT count(*) FROM intake_batches WHERE id != 'batch-valid'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count rejected batches"),
        0
    );
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT count(*) FROM intake_batch_items \
                 WHERE intake_batch_id = 'batch-invalid'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count rolled back items"),
        0
    );
}

#[test]
fn finalizes_an_exact_duplicate_once_without_creating_document_or_job_authority() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let source_path = root.path().join("statement.pdf");
    fs::write(&source_path, b"%PDF exact duplicate").expect("write source fixture");
    let first_input = import_input(&source_path, "document-existing", "audit-existing");
    store
        .register_import(&first_input, None)
        .expect("register original document");
    let items = [IntakeBatchItemInput {
        id: "item-duplicate",
        safe_input_label: "statement.pdf",
        acquisition_input_key: None,
        acquisition_input_version: None,
        retry_of_batch_item_id: None,
    }];
    store
        .create_intake_batch(&IntakeBatchInput {
            id: "batch-duplicate",
            acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
            items: &items,
        })
        .expect("create duplicate batch");
    let source = ManualImportStore::prepare_source_path("application/pdf", &source_path)
        .expect("prepare duplicate source");
    let capture = match store
        .source_capture_plan(
            &import_input(&source_path, "document-ignored", "audit-duplicate"),
            &source,
            None,
        )
        .expect("plan duplicate capture")
    {
        SourceCapturePlan::Capture(capture) => capture,
        SourceCapturePlan::RestoreConfirmationRequired(_) => panic!("available document"),
    };
    let stored = capture
        .store_prepared(&source)
        .expect("store duplicate envelope");
    let duplicate_input = import_input(&source_path, "document-ignored", "audit-duplicate");
    let outcome = store
        .persist_captured_intake_import(&duplicate_input, &stored, "item-duplicate")
        .expect("atomically register duplicate and terminal receipt");
    assert_eq!(outcome.document_id, "document-existing");
    assert_eq!(outcome.status, SourceDocumentImportStatus::AlreadyPresent);
    assert!(
        store
            .finalize_intake_batch_item(
                "item-duplicate",
                IntakeItemFinalization::Suppressed {
                    code: "duplicate_retry",
                },
            )
            .is_err(),
        "a terminal receipt cannot be rewritten"
    );
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM source_documents", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count documents"),
        1
    );
    assert_eq!(
        store
            .connection
            .query_row("SELECT count(*) FROM jobs", [], |row| row.get::<_, i64>(0))
            .expect("count jobs"),
        1
    );
    let receipt: (String, String) = store
        .connection
        .query_row(
            "SELECT capture_outcome, source_document_id FROM intake_batch_items \
             WHERE id = 'item-duplicate'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read duplicate receipt");
    assert_eq!(
        receipt,
        ("already_present".to_owned(), "document-existing".to_owned())
    );
}

#[test]
fn commits_new_document_audit_job_and_captured_receipt_in_one_transaction() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let source_path = root.path().join("captured.pdf");
    fs::write(&source_path, b"%PDF captured authority").expect("write source fixture");
    let items = [IntakeBatchItemInput {
        id: "item-captured",
        safe_input_label: "captured.pdf",
        acquisition_input_key: None,
        acquisition_input_version: None,
        retry_of_batch_item_id: None,
    }];
    store
        .create_intake_batch(&IntakeBatchInput {
            id: "batch-captured",
            acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
            items: &items,
        })
        .expect("create captured batch");
    let source = ManualImportStore::prepare_source_path("application/pdf", &source_path)
        .expect("prepare captured source");
    let input = import_input(&source_path, "document-captured", "audit-captured");
    let capture = match store
        .source_capture_plan(&input, &source, None)
        .expect("plan captured source")
    {
        SourceCapturePlan::Capture(capture) => capture,
        SourceCapturePlan::RestoreConfirmationRequired(_) => panic!("new document"),
    };
    let stored = capture
        .store_prepared(&source)
        .expect("store captured envelope");
    store
        .persist_captured_intake_import(&input, &stored, "item-captured")
        .expect("commit capture authority");

    let authority: (i64, i64, i64, String, String) = store
        .connection
        .query_row(
            "SELECT \
               (SELECT count(*) FROM source_documents), \
               (SELECT count(*) FROM audit_log), \
               (SELECT count(*) FROM jobs), \
               capture_outcome, source_document_id \
             FROM intake_batch_items WHERE id = 'item-captured'",
            [],
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
        .expect("read committed capture authority");
    assert_eq!(
        authority,
        (
            1,
            1,
            1,
            "captured".to_owned(),
            "document-captured".to_owned(),
        )
    );
}

#[test]
fn rolls_back_document_audit_job_and_receipt_when_atomic_terminalization_fails() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let source_path = root.path().join("rollback.pdf");
    fs::write(&source_path, b"%PDF rollback boundary").expect("write source fixture");
    let items = [IntakeBatchItemInput {
        id: "item-conflict",
        safe_input_label: "rollback.pdf",
        acquisition_input_key: None,
        acquisition_input_version: None,
        retry_of_batch_item_id: None,
    }];
    store
        .create_intake_batch(&IntakeBatchInput {
            id: "batch-rollback",
            acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
            items: &items,
        })
        .expect("create rollback batch");
    store
        .finalize_intake_batch_item(
            "item-conflict",
            IntakeItemFinalization::Suppressed { code: "cancelled" },
        )
        .expect("create a terminalization conflict");
    let source = ManualImportStore::prepare_source_path("application/pdf", &source_path)
        .expect("prepare rollback source");
    let input = import_input(&source_path, "document-rollback", "audit-rollback");
    let capture = match store
        .source_capture_plan(&input, &source, None)
        .expect("plan rollback capture")
    {
        SourceCapturePlan::Capture(capture) => capture,
        SourceCapturePlan::RestoreConfirmationRequired(_) => panic!("new document"),
    };
    let stored = capture
        .store_prepared(&source)
        .expect("store rollback envelope");
    assert!(
        store
            .persist_captured_intake_import(&input, &stored, "item-conflict")
            .is_err(),
        "receipt conflict must roll back all database authority"
    );
    for table in ["source_documents", "audit_log", "jobs"] {
        let count = store
            .connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count rolled back authority");
        assert_eq!(count, 0, "{table} must roll back");
    }
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT capture_outcome FROM intake_batch_items WHERE id = 'item-conflict'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("read unchanged receipt"),
        "suppressed"
    );
}

#[test]
fn restore_plan_terminalizes_the_receipt_before_restart_recovery() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let source_path = root.path().join("restore.pdf");
    fs::write(&source_path, b"%PDF restore confirmation").expect("write source fixture");
    {
        let mut store = open_store(root.path());
        store
            .register_import(
                &import_input(&source_path, "document-deleted", "audit-original"),
                None,
            )
            .expect("register original document");
        store
            .delete_source_document("document-deleted", "audit-delete")
            .expect("delete original document");
        let items = [IntakeBatchItemInput {
            id: "item-restore",
            safe_input_label: "restore.pdf",
            acquisition_input_key: None,
            acquisition_input_version: None,
            retry_of_batch_item_id: None,
        }];
        store
            .create_intake_batch(&IntakeBatchInput {
                id: "batch-restore",
                acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
                items: &items,
            })
            .expect("create restore batch");
        let source = ManualImportStore::prepare_source_path("application/pdf", &source_path)
            .expect("prepare deleted exact match");
        let plan = store
            .intake_source_capture_plan(
                &import_input(&source_path, "document-ignored", "audit-restore-attempt"),
                &source,
                "item-restore",
            )
            .expect("plan restore confirmation and finalize receipt");
        let SourceCapturePlan::RestoreConfirmationRequired(outcome) = plan else {
            panic!("deleted exact match must require confirmation");
        };
        assert_eq!(outcome.document_id, "document-deleted");
    }

    let store = ManualImportStore::open_existing(root.path(), Zeroizing::new(KEY))
        .expect("reopen encrypted Vault");
    let receipt: (String, String, Option<String>) = store
        .connection
        .query_row(
            "SELECT capture_outcome, source_document_id, rejection_code \
             FROM intake_batch_items WHERE id = 'item-restore'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read restore receipt after recovery");
    assert_eq!(
        receipt,
        (
            "restore_confirmation_required".to_owned(),
            "document-deleted".to_owned(),
            None,
        )
    );
}

#[test]
fn reopens_pending_receipts_into_truthful_channel_owned_outcomes() {
    let root = tempfile::tempdir().expect("temporary Vault");
    {
        let mut store = open_store(root.path());
        let explicit = [IntakeBatchItemInput {
            id: "item-explicit",
            safe_input_label: "manual.pdf",
            acquisition_input_key: None,
            acquisition_input_version: None,
            retry_of_batch_item_id: None,
        }];
        store
            .create_intake_batch(&IntakeBatchInput {
                id: "batch-explicit",
                acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
                items: &explicit,
            })
            .expect("create explicit batch");
        let local_key = "b".repeat(64);
        let local_version = format!("v1:{}", "c".repeat(64));
        let local = [local_item(
            "item-local",
            "local.pdf",
            &local_key,
            &local_version,
        )];
        store
            .create_intake_batch(&IntakeBatchInput {
                id: "batch-local",
                acquisition_channel: IntakeAcquisitionChannel::LocalInbox,
                items: &local,
            })
            .expect("create Local Inbox batch");
        let gmail = [IntakeBatchItemInput {
            id: "item-gmail",
            safe_input_label: "Transaction email",
            acquisition_input_key: None,
            acquisition_input_version: None,
            retry_of_batch_item_id: None,
        }];
        store
            .create_intake_batch(&IntakeBatchInput {
                id: "batch-gmail",
                acquisition_channel: IntakeAcquisitionChannel::Gmail,
                items: &gmail,
            })
            .expect("create Gmail batch");
    }

    let store = ManualImportStore::open_existing(root.path(), Zeroizing::new(KEY))
        .expect("reopen encrypted Vault");
    let recovered = store
        .connection
        .prepare(
            "SELECT id, capture_outcome, rejection_kind, rejection_code, finalized_at \
             FROM intake_batch_items ORDER BY id",
        )
        .expect("prepare recovered item query")
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .expect("query recovered items")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect recovered items");
    assert_eq!(recovered.len(), 3);
    assert_eq!(
        (
            recovered[0].0.as_str(),
            recovered[0].1.as_str(),
            recovered[0].2.as_deref(),
            recovered[0].3.as_deref(),
        ),
        (
            "item-explicit",
            "rejected",
            Some("background_action_required"),
            Some("handoff_interrupted"),
        )
    );
    assert_eq!(
        (&recovered[1].1, &recovered[1].2, &recovered[1].3),
        (
            &"suppressed".to_owned(),
            &None,
            &Some("discovery_retry".to_owned())
        )
    );
    assert_eq!(
        (&recovered[2].1, &recovered[2].2, &recovered[2].3),
        (
            &"suppressed".to_owned(),
            &None,
            &Some("discovery_retry".to_owned())
        )
    );
    assert!(recovered.iter().all(|item| item.4.is_some()));
}

#[test]
fn replacement_admission_resolves_only_the_matching_background_rejection() {
    let root = tempfile::tempdir().expect("temporary Vault");
    let mut store = open_store(root.path());
    let entry_key = "d".repeat(64);
    let first_version = format!("v1:{}", "e".repeat(64));
    let first = [local_item(
        "item-first",
        "statement.pdf",
        &entry_key,
        &first_version,
    )];
    store
        .create_intake_batch(&IntakeBatchInput {
            id: "batch-first",
            acquisition_channel: IntakeAcquisitionChannel::LocalInbox,
            items: &first,
        })
        .expect("create first attempt");
    store
        .finalize_intake_batch_item(
            "item-first",
            IntakeItemFinalization::Rejected {
                code: "capture_failed",
                kind: IntakeRejectionKind::BackgroundActionRequired,
                parked: true,
            },
        )
        .expect("reject first attempt");

    let unrelated_key = "f".repeat(64);
    let unrelated_version = format!("v1:{}", "a".repeat(64));
    let unrelated = [local_item(
        "item-unrelated",
        "other.pdf",
        &unrelated_key,
        &unrelated_version,
    )];
    store
        .create_intake_batch(&IntakeBatchInput {
            id: "batch-unrelated",
            acquisition_channel: IntakeAcquisitionChannel::LocalInbox,
            items: &unrelated,
        })
        .expect("create unrelated attempt");
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT rejection_resolved_at FROM intake_batch_items \
                 WHERE id = 'item-first'",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .expect("read unresolved rejection"),
        None
    );

    let replacement_version = format!("v1:{}", "b".repeat(64));
    let replacement = [local_item(
        "item-replacement",
        "renamed.pdf",
        &entry_key,
        &replacement_version,
    )];
    store
        .create_intake_batch(&IntakeBatchInput {
            id: "batch-replacement",
            acquisition_channel: IntakeAcquisitionChannel::LocalInbox,
            items: &replacement,
        })
        .expect("create correlated replacement");
    assert_eq!(
        store
            .connection
            .query_row(
                "SELECT rejection_resolution_kind, resolved_by_batch_item_id \
                 FROM intake_batch_items WHERE id = 'item-first'",
                [],
                |row| Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?
                )),
            )
            .expect("read superseded rejection"),
        (
            Some("superseded_by_new_attempt".to_owned()),
            Some("item-replacement".to_owned()),
        )
    );
}
