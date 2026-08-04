use super::*;
use crate::database::intake::{MoneySourceCandidateInput, MoneySourceCandidateScope};
use crate::database::{
    SourceDocumentRoutingOutcome, TrustedAccountCandidate, TrustedDocumentClassification,
    ValidatedExternalRecordInput, ValidatedStructuredParseInput,
};
use crate::runtime::tests::{
    MemoryLocalInboxBookmarkStore, MemoryRememberedKeyStore, MemoryStatementPasswordStore,
};
use std::fs;

fn runtime_fixture() -> (tempfile::TempDir, VaultRuntime) {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::with_secret_stores(
        parent.path().join("vault"),
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
        .seed_money_source("source-dbs", "dbs", "DBS", "bank")
        .expect("seed Money Source");
    (parent, runtime)
}

fn write_source(parent: &std::path::Path, name: &str, content: &[u8]) -> std::path::PathBuf {
    let path = parent.join(name);
    fs::write(&path, content).expect("write source");
    path
}

fn import(
    runtime: &VaultRuntime,
    source: &std::path::Path,
) -> crate::database::SourceDocumentImportOutcome {
    runtime.import_selected_document(source).expect("import")
}

fn start_parse(
    store: &mut crate::database::ManualImportStore,
    document_id: &str,
) -> crate::database::ParseDocumentClaim {
    let jobs = store.queued_parse_document_jobs().expect("list parse jobs");
    let job = jobs
        .into_iter()
        .find(|job| job.document_id == document_id)
        .expect("parse job for document");
    store
        .start_parse_document_job(&job)
        .expect("start parse job")
        .expect("claim parse job")
}

fn classify_document(
    store: &mut crate::database::ManualImportStore,
    document_id: &str,
) -> SourceDocumentRoutingOutcome {
    let input = TrustedDocumentClassification {
        accounts: &[TrustedAccountCandidate {
            account_id: "account-dbs",
            account_type: "checking",
            currency: Some("SGD"),
            display_name: "DBS Account",
            masked_identifier: None,
            provider_account_id: Some("1234"),
        }],
        audit_id: "audit-classify",
        document_id,
        document_type: Some("statement"),
        provider_key: "dbs",
        semantic_document_key: "dbs-2026-08",
        statement_period_from: Some("2026-08-01"),
        statement_period_to: Some("2026-08-31"),
    };
    store
        .apply_trusted_classification(&input)
        .expect("classify document")
}

fn finish_pipeline_to_ready(runtime: &VaultRuntime, document_id: &str) {
    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    let claim = start_parse(store, document_id);
    let outcome = classify_document(store, document_id);
    store
        .finish_parse_document_job(&claim, &outcome)
        .expect("finish parse job");
    store
        .start_reconcile_document(document_id)
        .expect("start reconcile job");
    store
        .reconcile_document(document_id)
        .expect("reconcile document");
    drop(store_guard);
}

fn local_inbox_key() -> (&'static str, &'static str) {
    (
        "0000000000000000000000000000000000000000000000000000000000000001",
        "v1:0000000000000000000000000000000000000000000000000000000000000001",
    )
}

fn add_local_inbox_item(
    store: &mut crate::database::ManualImportStore,
    batch_id: &str,
    item_id: &str,
    label: &str,
) {
    let (key, version) = local_inbox_key();
    let input = IntakeBatchInput {
        id: batch_id,
        acquisition_channel: IntakeAcquisitionChannel::LocalInbox,
        items: &[IntakeBatchItemInput {
            id: item_id,
            safe_input_label: label,
            acquisition_input_key: Some(key),
            acquisition_input_version: Some(version),
            retry_of_batch_item_id: None,
        }],
    };
    store.create_intake_batch(&input).expect("create batch");
}

fn add_explicit_handoff_item(
    store: &mut crate::database::ManualImportStore,
    batch_id: &str,
    item_id: &str,
    label: &str,
) {
    let input = IntakeBatchInput {
        id: batch_id,
        acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
        items: &[IntakeBatchItemInput {
            id: item_id,
            safe_input_label: label,
            acquisition_input_key: None,
            acquisition_input_version: None,
            retry_of_batch_item_id: None,
        }],
    };
    store.create_intake_batch(&input).expect("create batch");
}

fn row_with_consequence(rows: &[TaskRow], consequence: TaskConsequence) -> Option<&TaskRow> {
    rows.iter().find(|row| row.consequence == consequence)
}

#[test]
fn lists_processing_task_for_active_reconcile_job() {
    let (_parent, runtime) = runtime_fixture();
    let source = write_source(
        _parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let imported = import(&runtime, &source);

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    let claim = start_parse(store, &imported.document_id);
    let outcome = classify_document(store, &imported.document_id);
    store
        .finish_parse_document_job(&claim, &outcome)
        .expect("finish parse job");
    store
        .start_reconcile_document(&imported.document_id)
        .expect("start reconcile job");
    drop(store_guard);

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    let row =
        row_with_consequence(&tasks.rows, TaskConsequence::Processing).expect("processing row");
    assert_eq!(row.group, TaskGroup::InProgress);
    if let TaskDestination::Document { document_id } = &row.destination {
        assert_eq!(document_id, &imported.document_id);
    } else {
        panic!("expected Document destination");
    }
}

#[test]
fn lists_password_needed_task() {
    let (_parent, runtime) = runtime_fixture();
    let source = write_source(
        _parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let imported = import(&runtime, &source);

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    let claim = start_parse(store, &imported.document_id);
    classify_document(store, &imported.document_id);
    let outcome =
        SourceDocumentRoutingOutcome::needs_attention(&imported.document_id, "password_required");
    store
        .finish_parse_document_job(&claim, &outcome)
        .expect("finish parse job");
    drop(store_guard);

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    let row = row_with_consequence(&tasks.rows, TaskConsequence::PasswordNeeded)
        .expect("password needed row");
    assert_eq!(row.group, TaskGroup::NeedsAction);
    if let TaskDestination::Password { document_id, .. } = &row.destination {
        assert_eq!(document_id, &imported.document_id);
    } else {
        panic!("expected Password destination");
    }
}

#[test]
fn lists_new_source_detected_task() {
    let (_parent, runtime) = runtime_fixture();
    let source = write_source(
        _parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let imported = import(&runtime, &source);

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    let input = MoneySourceCandidateInput {
        candidate_id: "candidate-dbs",
        document_id: &imported.document_id,
        provider_key: "dbs",
        scope: MoneySourceCandidateScope::ProviderSingleton,
    };
    store
        .attach_money_source_candidate(&input)
        .expect("attach candidate");
    drop(store_guard);

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    let row = row_with_consequence(&tasks.rows, TaskConsequence::NewSourceDetected)
        .expect("new source row");
    assert_eq!(row.group, TaskGroup::NeedsAction);
    if let TaskDestination::SourceConfirmation {
        money_source_candidate_id,
    } = &row.destination
    {
        assert_eq!(money_source_candidate_id, "candidate-dbs");
    } else {
        panic!("expected SourceConfirmation destination");
    }
}

#[test]
fn lists_needs_review_task() {
    let (_parent, runtime) = runtime_fixture();
    let source = write_source(
        _parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let imported = import(&runtime, &source);

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    let claim = start_parse(store, &imported.document_id);

    // Classify first so the account exists for the structured parse records.
    let outcome = classify_document(store, &imported.document_id);

    let parse = ValidatedStructuredParseInput {
        normalization_profile_id: "profile-1".to_string(),
        profile_json: "{}".to_string(),
        records: vec![ValidatedExternalRecordInput {
            account_id: "account-dbs".to_string(),
            account_balance_delta: Some("0.00".to_string()),
            amount_value: Some("1.00".to_string()),
            currency: Some("SGD".to_string()),
            event_type: Some("transfer".to_string()),
            posted_on: Some("2026-08-01".to_string()),
            posting_status: Some("posted".to_string()),
            raw_json: "{}".to_string(),
            record_type: "transaction".to_string(),
            stable_record_key: "record-1".to_string(),
            validation_json:
                r#"{"schemaValid":true,"rawGrounded":true,"deterministicValidationPassed":true}"#
                    .to_string(),
        }],
    };
    store
        .persist_validated_structured_parse_for_claimed_job(
            &imported.document_id,
            &parse,
            &claim,
            "input-hash",
            "output-hash",
        )
        .expect("persist parse");

    store
        .finish_parse_document_job(&claim, &outcome)
        .expect("finish parse job");
    store
        .start_reconcile_document(&imported.document_id)
        .expect("start reconcile job");
    store
        .reconcile_document(&imported.document_id)
        .expect("reconcile document");
    drop(store_guard);

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    let row =
        row_with_consequence(&tasks.rows, TaskConsequence::NeedsReview).expect("needs review row");
    assert_eq!(row.group, TaskGroup::NeedsAction);
    if let TaskDestination::ReviewGroup { document_id } = &row.destination {
        assert_eq!(document_id, &imported.document_id);
    } else {
        panic!("expected ReviewGroup destination");
    }
}

#[test]
fn lists_needs_attention_task() {
    let (_parent, runtime) = runtime_fixture();
    let source = write_source(
        _parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let imported = import(&runtime, &source);

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    let claim = start_parse(store, &imported.document_id);
    let outcome = SourceDocumentRoutingOutcome::needs_attention(
        &imported.document_id,
        "money_source_not_found",
    );
    store
        .finish_parse_document_job(&claim, &outcome)
        .expect("finish parse job");
    drop(store_guard);

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    let row = row_with_consequence(&tasks.rows, TaskConsequence::NeedsAttention)
        .expect("needs attention row");
    assert_eq!(row.group, TaskGroup::NeedsAction);
    if let TaskDestination::Document { document_id } = &row.destination {
        assert_eq!(document_id, &imported.document_id);
    } else {
        panic!("expected Document destination");
    }
}

#[test]
fn lists_ready_task_after_completed_pipeline() {
    let (_parent, runtime) = runtime_fixture();
    let source = write_source(
        _parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    let row = row_with_consequence(&tasks.rows, TaskConsequence::Ready).expect("ready row");
    assert_eq!(row.group, TaskGroup::RecentlyCompleted);
    if let TaskDestination::Receipt { intake_item_id } = &row.destination {
        assert_eq!(
            intake_item_id.as_str(),
            imported.intake_item_id.as_deref().unwrap_or("")
        );
    } else {
        panic!("expected Receipt destination");
    }
}

#[test]
fn lists_restore_source_file_task() {
    let (_parent, runtime) = runtime_fixture();
    let source = write_source(
        _parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let imported = import(&runtime, &source);
    runtime
        .delete_source_document(&imported.document_id)
        .expect("delete source");
    let confirmation = import(&runtime, &source);
    assert_eq!(
        confirmation.status,
        crate::database::SourceDocumentImportStatus::RestoreConfirmationRequired
    );

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    let row =
        row_with_consequence(&tasks.rows, TaskConsequence::RestoreSourceFile).expect("restore row");
    assert_eq!(row.group, TaskGroup::NeedsAction);
    if let TaskDestination::Document { document_id } = &row.destination {
        assert_eq!(document_id, &imported.document_id);
    } else {
        panic!("expected Document destination");
    }
}

#[test]
fn lists_source_file_restored_and_left_deleted_tasks() {
    let (_parent, runtime) = runtime_fixture();
    let source = write_source(
        _parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let imported = import(&runtime, &source);
    runtime
        .delete_source_document(&imported.document_id)
        .expect("delete source");

    let confirmation = import(&runtime, &source);
    let confirm_item_id = confirmation
        .intake_item_id
        .as_deref()
        .expect("restore receipt id");
    let restored = runtime
        .confirm_restore_selected_document(&source, &imported.document_id, confirm_item_id)
        .expect("restore source");
    assert_eq!(
        restored.status,
        crate::database::SourceDocumentImportStatus::Restored
    );

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    assert!(
        row_with_consequence(&tasks.rows, TaskConsequence::SourceFileRestored).is_some(),
        "source file restored row"
    );

    runtime
        .delete_source_document(&imported.document_id)
        .expect("delete source again");
    let confirmation2 = import(&runtime, &source);
    let confirm_item_id2 = confirmation2
        .intake_item_id
        .as_deref()
        .expect("restore receipt id");
    runtime
        .decline_restore_selected_document(&imported.document_id, confirm_item_id2)
        .expect("decline restore");

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    assert!(
        row_with_consequence(&tasks.rows, TaskConsequence::SourceFileLeftDeleted).is_some(),
        "source file left deleted row"
    );
}

#[test]
fn lists_inbox_and_import_interrupted_tasks() {
    let (_parent, runtime) = runtime_fixture();

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");

    add_local_inbox_item(store, "batch-inbox", "item-inbox", "inbox-file.pdf");
    store
        .finalize_intake_batch_item(
            "item-inbox",
            IntakeItemFinalization::Rejected {
                code: "import_failed",
                kind: IntakeRejectionKind::BackgroundActionRequired,
                parked: false,
            },
        )
        .expect("finalize inbox item");

    add_explicit_handoff_item(store, "batch-handoff", "item-handoff", "handoff-file.pdf");
    store
        .finalize_intake_batch_item(
            "item-handoff",
            IntakeItemFinalization::Rejected {
                code: "handoff_interrupted",
                kind: IntakeRejectionKind::BackgroundActionRequired,
                parked: true,
            },
        )
        .expect("finalize handoff item");
    drop(store_guard);

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    assert!(row_with_consequence(&tasks.rows, TaskConsequence::InboxFileCouldNotBeAdded).is_some());
    assert!(row_with_consequence(&tasks.rows, TaskConsequence::ImportInterrupted).is_some());
}

#[test]
fn lists_already_in_cancan_and_file_not_added_tasks() {
    let (_parent, runtime) = runtime_fixture();
    let source = write_source(
        _parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let _ = import(&runtime, &source);
    let duplicate = import(&runtime, &source);
    assert_eq!(
        duplicate.status,
        crate::database::SourceDocumentImportStatus::AlreadyPresent
    );

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    add_explicit_handoff_item(store, "batch-rejected", "item-rejected", "rejected.pdf");
    store
        .finalize_intake_batch_item(
            "item-rejected",
            IntakeItemFinalization::Rejected {
                code: "unsupported_document",
                kind: IntakeRejectionKind::VisibleReceipt,
                parked: false,
            },
        )
        .expect("finalize visible rejection");
    drop(store_guard);

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    assert!(row_with_consequence(&tasks.rows, TaskConsequence::AlreadyInCancan).is_some());
    assert!(row_with_consequence(&tasks.rows, TaskConsequence::FileNotAdded).is_some());
}

#[test]
fn lists_parked_tasks() {
    let (_parent, runtime) = runtime_fixture();

    // Source unassigned parked: attach a candidate and keep it unassigned.
    let source = write_source(
        _parent.path(),
        "unassigned.pdf",
        b"%PDF-1.4\nsynthetic unassigned",
    );
    let unassigned = import(&runtime, &source);
    {
        let mut store_guard = runtime.store().expect("open store");
        let store = store_guard.as_mut().expect("unlocked store");
        let input = MoneySourceCandidateInput {
            candidate_id: "candidate-dbs",
            document_id: &unassigned.document_id,
            provider_key: "dbs",
            scope: MoneySourceCandidateScope::ProviderSingleton,
        };
        store
            .attach_money_source_candidate(&input)
            .expect("attach candidate");
        store
            .keep_money_source_candidate_unassigned("candidate-dbs", 1)
            .expect("keep unassigned");
        drop(store_guard);
    }

    // Password parked: classify, finish pipeline, then park the password step.
    let source = write_source(
        _parent.path(),
        "password.pdf",
        b"%PDF-1.4\nsynthetic password",
    );
    let password_doc = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &password_doc.document_id);
    {
        let mut store_guard = runtime.store().expect("open store");
        let store = store_guard.as_mut().expect("unlocked store");
        store
            .set_source_document_attention_parked_for_test(
                &password_doc.document_id,
                "statement_password",
                "2026-08-10T00:00:00Z",
            )
            .expect("park password");

        // Inbox file parked
        add_local_inbox_item(store, "batch-parked", "item-parked", "parked-file.pdf");
        store
            .finalize_intake_batch_item(
                "item-parked",
                IntakeItemFinalization::Rejected {
                    code: "import_failed",
                    kind: IntakeRejectionKind::BackgroundActionRequired,
                    parked: true,
                },
            )
            .expect("finalize parked inbox");
        drop(store_guard);
    }

    let tasks = runtime.list_tasks(TaskFilter::Full).expect("list tasks");
    assert!(row_with_consequence(&tasks.rows, TaskConsequence::SourceUnassigned).is_some());
    assert!(row_with_consequence(&tasks.rows, TaskConsequence::PasswordParked).is_some());
    assert!(row_with_consequence(&tasks.rows, TaskConsequence::InboxFileParked).is_some());

    let command_center = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    assert!(
        row_with_consequence(&command_center.rows, TaskConsequence::SourceUnassigned).is_none()
    );
    assert!(row_with_consequence(&command_center.rows, TaskConsequence::PasswordParked).is_none());
    assert!(row_with_consequence(&command_center.rows, TaskConsequence::InboxFileParked).is_none());
}

#[test]
fn command_center_truncates_to_five_rows_and_counts_all_needs_action() {
    let (parent, runtime) = runtime_fixture();
    let recovery_file = parent.path().join("recovery.cancan-recovery");
    runtime
        .save_recovery_file(&recovery_file)
        .expect("save recovery file");

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    for i in 0..6 {
        let batch_id = format!("batch-{i}");
        let item_id = format!("item-{i}");
        let label = format!("file-{i}.pdf");
        let timestamp = format!("2025-08-{:02}T00:00:00Z", i + 1);
        store
            .insert_local_inbox_rejected_batch_for_test(
                &batch_id,
                &item_id,
                &label,
                &timestamp,
                "import_failed",
            )
            .expect("insert rejected inbox item");
    }
    drop(store_guard);

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    assert_eq!(tasks.needs_action_count, 6);
    assert_eq!(tasks.rows.len(), 5);
    assert_eq!(tasks.rows[0].title, "file-0.pdf");
    assert_eq!(tasks.rows[4].title, "file-4.pdf");
}

#[test]
fn recently_completed_tasks_expire_after_168_hours() {
    let (_parent, runtime) = runtime_fixture();

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    store
        .insert_visible_receipt_batch_for_test(
            "batch-old",
            "item-old",
            "old-receipt.pdf",
            "2025-01-01T00:00:00Z",
        )
        .expect("insert old visible receipt");
    store
        .insert_visible_receipt_batch_for_test(
            "batch-new",
            "item-new",
            "new-receipt.pdf",
            "2030-01-01T00:00:00Z",
        )
        .expect("insert new visible receipt");
    drop(store_guard);

    let tasks = runtime.list_tasks(TaskFilter::Full).expect("list tasks");
    let file_not_added = row_with_consequence(&tasks.rows, TaskConsequence::FileNotAdded)
        .expect("new visible receipt row");
    assert_eq!(file_not_added.title, "new-receipt.pdf");
    assert!(tasks.rows.iter().all(|row| row.title != "old-receipt.pdf"));
}

#[test]
fn reconcile_sealed_batches_completes_actionable_and_suppresses_non_actionable() {
    let (_parent, runtime) = runtime_fixture();

    // Actionable captured batch: explicit handoff with a file that becomes Ready.
    let source = write_source(_parent.path(), "ready.pdf", b"%PDF-1.4\nsynthetic ready");
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);

    // Duplicate-only batch.
    let _ = import(&runtime, &source);

    // Visible rejection-only batch.
    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    store
        .insert_visible_receipt_batch_for_test(
            "batch-rejected",
            "item-rejected",
            "rejected.pdf",
            "2026-08-10T00:00:00Z",
        )
        .expect("insert visible rejection");
    drop(store_guard);

    // list_tasks reconciles all sealed batches.
    let _ = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    let states = store
        .intake_batch_completion_for_test()
        .expect("batch states");
    drop(store_guard);

    assert!(
        states
            .iter()
            .any(|(_, completed, state)| { completed.is_some() && state == "pending" })
    );
    assert!(states.iter().all(|(_, completed, _)| completed.is_some()));
}

#[test]
fn recovery_setup_tasks() {
    let (parent, runtime) = runtime_fixture();

    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    let row = row_with_consequence(&tasks.rows, TaskConsequence::SaveRecoveryFile)
        .expect("save recovery file row");
    assert_eq!(row.group, TaskGroup::NeedsAction);

    runtime
        .postpone_recovery_setup()
        .expect("postpone recovery setup");

    let tasks = runtime.list_tasks(TaskFilter::Full).expect("list tasks");
    assert!(row_with_consequence(&tasks.rows, TaskConsequence::SetupReminderPostponed).is_some());
    assert!(
        row_with_consequence(&tasks.rows, TaskConsequence::SaveRecoveryFile).is_none(),
        "postponed reminder hides save recovery file"
    );

    // Saving a recovery file overwrites the postpone marker with a configured status.
    let recovery_file = parent.path().join("recovery.cancan-recovery");
    runtime
        .save_recovery_file(&recovery_file)
        .expect("save recovery file");
    let status = fs::read(parent.path().join("vault").join("vault-recovery.status"))
        .expect("recovery status");
    assert!(&status[..8] == b"CCRECST1" || &status[..8] == b"CCRECST2");
}
