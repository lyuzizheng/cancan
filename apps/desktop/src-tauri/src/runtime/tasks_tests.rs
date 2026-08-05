use super::*;
use crate::database::intake::{MoneySourceCandidateInput, MoneySourceCandidateScope};
use crate::database::{SourceDocumentRoutingOutcome, ValidatedStructuredParseInput};

use tasks_test_support::*;

#[test]
fn lists_processing_task_for_active_reconcile_job() {
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
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
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
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
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
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
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
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
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
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
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
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
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
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
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
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
    let restored_row = row_with_consequence(&tasks.rows, TaskConsequence::SourceFileRestored)
        .expect("source file restored row");
    // Fix #1 regression guard: the audit correlation is per intake item, so the
    // restored row must point at this cycle's confirmation item.
    if let TaskDestination::Receipt { intake_item_id } = &restored_row.destination {
        assert_eq!(intake_item_id, confirm_item_id);
    } else {
        panic!("expected Receipt destination");
    }

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
    let left_deleted_row =
        row_with_consequence(&tasks.rows, TaskConsequence::SourceFileLeftDeleted)
            .expect("source file left deleted row");
    if let TaskDestination::Receipt { intake_item_id } = &left_deleted_row.destination {
        assert_eq!(intake_item_id, confirm_item_id2);
    } else {
        panic!("expected Receipt destination");
    }
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
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
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
    let (parent, runtime) = runtime_fixture();

    // Source unassigned parked: attach a candidate and keep it unassigned.
    let source = write_source(
        parent.path(),
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
        parent.path(),
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
                "2026-08-10 00:00:00",
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
        // CURRENT_TIMESTAMP format, as production writes it.
        let timestamp = format!("2025-08-{:02} 00:00:00", i + 1);
        store
            .insert_local_inbox_rejected_batch_for_test(
                &batch_id,
                &item_id,
                &label,
                &timestamp,
                "import_failed",
                false,
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
            "2025-01-01 00:00:00",
        )
        .expect("insert old visible receipt");
    store
        .insert_visible_receipt_batch_for_test(
            "batch-new",
            "item-new",
            "new-receipt.pdf",
            "2030-01-01 00:00:00",
        )
        .expect("insert new visible receipt");
    drop(store_guard);

    let tasks = runtime.list_tasks(TaskFilter::Full).expect("list tasks");
    let file_not_added = row_with_consequence(&tasks.rows, TaskConsequence::FileNotAdded)
        .expect("new visible receipt row");
    assert_eq!(file_not_added.title, "new-receipt.pdf");
    assert!(tasks.rows.iter().all(|row| row.title != "old-receipt.pdf"));
}
