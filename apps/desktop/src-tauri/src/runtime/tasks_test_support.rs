use super::*;
use crate::database::{
    SourceDocumentRoutingOutcome, TrustedAccountCandidate, TrustedDocumentClassification,
};
use crate::runtime::tests::{
    MemoryLocalInboxBookmarkStore, MemoryRememberedKeyStore, MemoryStatementPasswordStore,
};
use std::fs;

pub(super) fn runtime_fixture() -> (tempfile::TempDir, VaultRuntime) {
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

pub(super) fn write_source(
    parent: &std::path::Path,
    name: &str,
    content: &[u8],
) -> std::path::PathBuf {
    let path = parent.join(name);
    fs::write(&path, content).expect("write source");
    path
}

pub(super) fn import(
    runtime: &VaultRuntime,
    source: &std::path::Path,
) -> crate::database::SourceDocumentImportOutcome {
    runtime.import_selected_document(source).expect("import")
}

pub(super) fn start_parse(
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

pub(super) fn classify_document(
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
        provider_root_id: None,
        semantic_document_key: "dbs-2026-08",
        statement_period_from: Some("2026-08-01"),
        statement_period_to: Some("2026-08-31"),
    };
    store
        .apply_trusted_classification(&input)
        .expect("classify document")
}

pub(super) fn finish_pipeline_to_ready(runtime: &VaultRuntime, document_id: &str) {
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

pub(super) fn local_inbox_key() -> (&'static str, &'static str) {
    (
        "0000000000000000000000000000000000000000000000000000000000000001",
        "v1:0000000000000000000000000000000000000000000000000000000000000001",
    )
}

pub(super) fn add_local_inbox_item(
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

pub(super) fn add_explicit_handoff_item(
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

pub(super) fn row_with_consequence(
    rows: &[TaskRow],
    consequence: TaskConsequence,
) -> Option<&TaskRow> {
    rows.iter().find(|row| row.consequence == consequence)
}

#[test]
fn reconcile_sealed_batches_completes_actionable_and_suppresses_non_actionable() {
    let (parent, runtime) = runtime_fixture();

    // Actionable captured batch: explicit handoff with a file that becomes Ready.
    let source = write_source(parent.path(), "ready.pdf", b"%PDF-1.4\nsynthetic ready");
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
            "2026-08-10 00:00:00",
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

    // Only the actionable captured batch notifies; the duplicate-only and
    // visible-rejection-only batches are suppressed.
    let pending: Vec<_> = states
        .iter()
        .filter(|(_, completed, state)| completed.is_some() && state == "pending")
        .collect();
    assert_eq!(pending.len(), 1);
    let rejected_batch = states
        .iter()
        .find(|(id, _, _)| id == "batch-rejected")
        .expect("batch-rejected present");
    assert!(rejected_batch.1.is_some());
    assert_eq!(rejected_batch.2, "suppressed");
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
    assert!(&status[..8] == b"CCRECST1");
    let tasks = runtime.list_tasks(TaskFilter::Full).expect("list tasks");
    assert!(
        row_with_consequence(&tasks.rows, TaskConsequence::SetupReminderPostponed).is_none(),
        "configured without a reminder shows no recovery row"
    );

    // S8 regression guard: the row keys off the reminder, not recovery_configured(),
    // so postponing after the recovery file is saved still renders the postponed row.
    runtime
        .postpone_recovery_setup()
        .expect("postpone after save");
    let tasks = runtime.list_tasks(TaskFilter::Full).expect("list tasks");
    let row = row_with_consequence(&tasks.rows, TaskConsequence::SetupReminderPostponed)
        .expect("postponed row after save + postpone");
    assert_eq!(row.group, TaskGroup::Parked);
    assert!(
        row_with_consequence(&tasks.rows, TaskConsequence::SaveRecoveryFile).is_none(),
        "postponed reminder hides save recovery file"
    );
}

#[test]
fn lists_needs_attention_after_failed_reconcile_with_money_source_assigned() {
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
    store
        .fail_reconcile_document(&imported.document_id, "provider_unavailable")
        .expect("fail reconcile");
    drop(store_guard);

    // Fix #4 regression guard: reconcile fails after the money source is already
    // assigned, so the old money_source_id filter would have hidden this row.
    let tasks = runtime
        .list_tasks(TaskFilter::CommandCenter)
        .expect("list tasks");
    let row = row_with_consequence(&tasks.rows, TaskConsequence::NeedsAttention)
        .expect("needs attention row after failed reconcile");
    assert_eq!(row.group, TaskGroup::NeedsAction);
    if let TaskDestination::Document { document_id } = &row.destination {
        assert_eq!(document_id, &imported.document_id);
    } else {
        panic!("expected Document destination");
    }
}

#[test]
fn ready_task_expires_168_hours_from_reconcile_finish() {
    let (parent, runtime) = runtime_fixture();
    let source = write_source(
        parent.path(),
        "statement.pdf",
        b"%PDF-1.4\nsynthetic statement",
    );
    let imported = import(&runtime, &source);
    finish_pipeline_to_ready(&runtime, &imported.document_id);

    // Backdate the reconcile finish past the 168-hour retention. The intake
    // capture happened "now", so this exercises the retention anchor (Fix #2/#3):
    // retention runs from the reconcile job's finished_at, not capture time.
    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    store
        .backdate_reconcile_finished_at_for_test(&imported.document_id, "2020-01-01 00:00:00")
        .expect("backdate reconcile finish");
    drop(store_guard);

    let tasks = runtime.list_tasks(TaskFilter::Full).expect("list tasks");
    assert!(
        row_with_consequence(&tasks.rows, TaskConsequence::Ready).is_none(),
        "Ready receipt must expire 168 hours after the reconcile finish"
    );
}

#[test]
fn parked_local_inbox_rejection_is_suppressed_not_pending() {
    let (_parent, runtime) = runtime_fixture();

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    // Fix #5 regression guard: a parked local-inbox rejection surfaces only in
    // the Parked group, so its batch must be `suppressed`, not `pending`.
    store
        .insert_local_inbox_rejected_batch_for_test(
            "batch-parked-inbox",
            "item-parked-inbox",
            "parked.pdf",
            "2026-08-01 00:00:00",
            "import_failed",
            true,
        )
        .expect("insert parked local inbox rejection");
    drop(store_guard);

    let tasks = runtime.list_tasks(TaskFilter::Full).expect("list tasks");
    assert!(
        row_with_consequence(&tasks.rows, TaskConsequence::InboxFileParked).is_some(),
        "parked inbox row renders in the Parked group"
    );
    assert!(
        row_with_consequence(&tasks.rows, TaskConsequence::InboxFileCouldNotBeAdded).is_none(),
        "parked inbox item must not also render as could-not-be-added"
    );

    let mut store_guard = runtime.store().expect("open store");
    let store = store_guard.as_mut().expect("unlocked store");
    let states = store
        .intake_batch_completion_for_test()
        .expect("batch states");
    drop(store_guard);
    let parked_batch = states
        .iter()
        .find(|(id, _, _)| id == "batch-parked-inbox")
        .expect("parked batch present");
    assert!(parked_batch.1.is_some(), "parked batch is completed");
    assert_eq!(parked_batch.2, "suppressed");
}
