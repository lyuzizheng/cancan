use super::inbox_intake::local_inbox_snapshot_version;
use super::tests::{
    MemoryLocalInboxBookmarkStore, MemoryRememberedKeyStore, MemoryStatementPasswordStore,
};
use super::*;
use crate::database::SourceDocumentImportStatus;
use crate::local_inbox::{FileIdentity, FileSnapshot};
#[cfg(target_os = "macos")]
use crate::viewer::tests::synthetic_png_fixture;
#[cfg(unix)]
use std::{ffi::OsString, os::unix::ffi::OsStringExt};
use std::{
    sync::{Arc, Barrier, mpsc},
    thread,
    time::Duration,
};

#[test]
fn local_inbox_snapshot_version_uses_fixed_width_signed_and_unsigned_fields() {
    let base = FileSnapshot {
        change_nanoseconds: 2,
        change_seconds: 3,
        creation_nanoseconds: 4,
        creation_seconds: 5,
        identity: FileIdentity {
            device: 6,
            inode: 7,
        },
        size: 8,
        modified_nanoseconds: 9,
        modified_seconds: 10,
    };
    let variants = [
        base.clone(),
        FileSnapshot {
            creation_seconds: i64::MIN,
            ..base.clone()
        },
        FileSnapshot {
            creation_seconds: i64::MAX,
            ..base.clone()
        },
        FileSnapshot {
            identity: FileIdentity {
                device: u64::MAX,
                ..base.identity
            },
            ..base.clone()
        },
        FileSnapshot {
            identity: FileIdentity {
                inode: u64::MAX,
                ..base.identity
            },
            ..base.clone()
        },
        FileSnapshot {
            size: u64::MAX,
            ..base
        },
    ];
    let versions = variants
        .iter()
        .map(local_inbox_snapshot_version)
        .collect::<Vec<_>>();
    assert!(versions.iter().all(|version| {
        version.starts_with("v1:")
            && version.len() == 67
            && version[3..].bytes().all(|byte| byte.is_ascii_hexdigit())
    }));
    for (index, version) in versions.iter().enumerate() {
        assert!(
            versions[index + 1..].iter().all(|other| other != version),
            "snapshot digest collision at variant {index}"
        );
    }
}

#[cfg(unix)]
#[test]
fn explicit_ineligible_inputs_create_visible_terminal_receipts() {
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
    let missing = parent.path().join("missing.pdf");
    let unsupported = parent.path().join("notes.txt");
    std::fs::write(&unsupported, b"not a supported evidence type").expect("write text file");
    let non_utf8 = parent
        .path()
        .join(OsString::from_vec(b"statement.\xffpdf".to_vec()));

    let missing_error = runtime
        .import_selected_document(&missing)
        .expect_err("missing input must fail");
    assert_eq!(missing_error.code(), "import_failed");
    let unsupported_error = runtime
        .import_selected_document(&unsupported)
        .expect_err("unsupported input must fail");
    assert_eq!(unsupported_error.code(), "unsupported_document");
    let non_utf8_error = runtime
        .import_selected_document(&non_utf8)
        .expect_err("non-UTF-8 input must fail");
    assert_eq!(non_utf8_error.code(), "import_failed");

    let store = runtime.store().expect("open store");
    let store = store.as_ref().expect("unlocked store");
    let items = store.intake_item_test_states().expect("read receipts");
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|item| {
        item.capture_outcome == "rejected"
            && item.rejection_kind.as_deref() == Some("visible_receipt")
            && item.rejection_resolved_at.is_none()
            && !item.safe_input_label.contains('/')
    }));
    let mut codes = items
        .iter()
        .filter_map(|item| item.rejection_code.as_deref())
        .collect::<Vec<_>>();
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec!["import_failed", "import_failed", "unsupported_document"]
    );
}

#[test]
fn explicit_plan_failure_releases_store_before_finalization() {
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
    let source = parent.path().join("plan-failure.pdf");
    std::fs::write(&source, b"%PDF-1.4\nplan failure").expect("write source");
    runtime.inject_intake_plan_failure();
    let (sender, receiver) = mpsc::channel();
    let worker = runtime.clone();
    thread::spawn(move || {
        sender
            .send(worker.import_selected_document(&source))
            .expect("send plan result");
    });
    let error = receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("plan failure must not deadlock")
        .expect_err("injected plan failure");
    assert_eq!(error.code(), "import_failed");
    let store = runtime.store().expect("open store");
    let store = store.as_ref().expect("unlocked store");
    let items = store.intake_item_test_states().expect("read receipt");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].capture_outcome, "rejected");
    assert_eq!(items[0].rejection_code.as_deref(), Some("import_failed"));
}

#[test]
fn explicit_persist_failure_releases_store_before_finalization() {
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
    let source = parent.path().join("persist-failure.pdf");
    std::fs::write(&source, b"%PDF-1.4\npersist failure").expect("write source");
    runtime.inject_intake_persist_failure();
    let (sender, receiver) = mpsc::channel();
    let worker = runtime.clone();
    thread::spawn(move || {
        sender
            .send(worker.import_selected_document(&source))
            .expect("send persist result");
    });
    let error = receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("persist failure must not deadlock")
        .expect_err("injected persist failure");
    assert_eq!(error.code(), "import_failed");
    let store = runtime.store().expect("open store");
    let store = store.as_ref().expect("unlocked store");
    let items = store.intake_item_test_states().expect("read receipt");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].capture_outcome, "rejected");
    assert_eq!(items[0].rejection_code.as_deref(), Some("import_failed"));
}

#[test]
fn explicit_finalization_failure_is_returned_without_claiming_terminalization() {
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
    runtime.inject_intake_finalization_failure();
    let error = runtime
        .import_selected_document(&parent.path().join("missing.pdf"))
        .expect_err("missing input must fail");
    assert_eq!(error.code(), "intake_finalization_failed");
    let store = runtime.store().expect("open store");
    let store = store.as_ref().expect("unlocked store");
    let items = store.intake_item_test_states().expect("read receipt");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].capture_outcome, "pending");
}

#[test]
fn explicit_imports_terminalize_capture_duplicate_restore_and_retry_receipts() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let vault_root = parent.path().join("vault");
    let runtime = VaultRuntime::with_secret_stores(
        vault_root,
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let source = parent.path().join("statement.pdf");
    std::fs::write(&source, b"%PDF-1.4\nsynthetic statement").expect("write synthetic statement");

    let imported = runtime
        .import_selected_document(&source)
        .expect("import statement");
    assert_eq!(imported.status, SourceDocumentImportStatus::Imported);
    let duplicate = runtime
        .import_selected_document(&source)
        .expect("import duplicate statement");
    assert_eq!(duplicate.status, SourceDocumentImportStatus::AlreadyPresent);

    runtime
        .delete_source_document(&imported.document_id)
        .expect("delete source file");
    let restore_confirmation = runtime
        .import_selected_document(&source)
        .expect("inspect deleted exact duplicate");
    assert_eq!(
        restore_confirmation.status,
        SourceDocumentImportStatus::RestoreConfirmationRequired
    );
    let receipt_id = restore_confirmation
        .intake_item_id
        .as_deref()
        .expect("restore receipt id");
    let restored = runtime
        .confirm_restore_selected_document(&source, &imported.document_id, receipt_id)
        .expect("restore source file");
    assert_eq!(restored.status, SourceDocumentImportStatus::Restored);
    let restored_again = runtime
        .confirm_restore_selected_document(&source, &imported.document_id, receipt_id)
        .expect("retry restore decision");
    assert_eq!(restored_again.status, SourceDocumentImportStatus::Restored);

    let store = runtime.store().expect("open store");
    let store = store.as_ref().expect("unlocked store");
    let batches = store
        .intake_batch_test_states()
        .expect("read intake batches");
    assert_eq!(batches.len(), 3);
    assert!(batches.iter().all(|batch| {
        batch.acquisition_channel == "explicit_handoff"
            && batch.item_count == 1
            && batch.sealed_at.is_some()
    }));
    let items = store.intake_item_test_states().expect("read intake items");
    let mut outcomes = items
        .iter()
        .map(|item| item.capture_outcome.as_str())
        .collect::<Vec<_>>();
    outcomes.sort_unstable();
    assert_eq!(
        outcomes,
        vec![
            "already_present",
            "captured",
            "restore_confirmation_required"
        ]
    );
    let restore_receipt = items
        .iter()
        .find(|item| item.id == receipt_id)
        .expect("restore receipt");
    assert_eq!(
        restore_receipt.capture_outcome,
        "restore_confirmation_required"
    );
    assert_eq!(
        restore_receipt.source_document_id.as_deref(),
        Some(imported.document_id.as_str())
    );
    assert!(items.iter().all(|item| item.rejection_code.is_none()));
    let document = store
        .list_unassigned_documents()
        .expect("list restored source")
        .into_iter()
        .find(|document| document.document_id == imported.document_id)
        .expect("restored source document");
    assert_eq!(document.file_state, "available");
    assert!(
        store
            .source_document_audit_actions_test(&imported.document_id)
            .expect("read restore decision audit")
            .contains(&"source_file_restored".to_owned())
    );
}

#[test]
fn restore_decision_is_mutually_exclusive_and_decline_is_idempotent() {
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
    let source = parent.path().join("declined-statement.pdf");
    std::fs::write(&source, b"%PDF-1.4\ndecline decision").expect("write statement");
    let imported = runtime
        .import_selected_document(&source)
        .expect("import statement");
    runtime
        .delete_source_document(&imported.document_id)
        .expect("delete source file");
    let confirmation = runtime
        .import_selected_document(&source)
        .expect("create restore decision receipt");
    let receipt_id = confirmation
        .intake_item_id
        .as_deref()
        .expect("restore receipt id");

    runtime
        .decline_restore_selected_document(&imported.document_id, receipt_id)
        .expect("decline restore");
    runtime
        .decline_restore_selected_document(&imported.document_id, receipt_id)
        .expect("retry decline decision");
    let error = runtime
        .confirm_restore_selected_document(&source, &imported.document_id, receipt_id)
        .expect_err("declined receipt must not be confirmable");
    assert_eq!(error.code(), "import_failed");

    let store = runtime.store().expect("open store");
    let store = store.as_ref().expect("unlocked store");
    let actions = store
        .source_document_audit_actions_test(&imported.document_id)
        .expect("read decision audits");
    assert_eq!(
        actions
            .iter()
            .filter(|action| *action == "source_file_restore_declined")
            .count(),
        1
    );
    assert!(
        !actions
            .iter()
            .any(|action| action == "source_file_restored")
    );
    let document = store
        .list_unassigned_documents()
        .expect("list declined source")
        .into_iter()
        .find(|document| document.document_id == imported.document_id)
        .expect("declined source document");
    assert_eq!(document.file_state, "deleted");
}

#[test]
fn interrupted_explicit_intake_is_terminal_after_store_restart() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let vault_root = parent.path().join("vault");
    let runtime = VaultRuntime::with_secret_stores(
        vault_root.clone(),
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    {
        let mut store = runtime.store().expect("open store");
        store
            .as_mut()
            .expect("unlocked store")
            .create_intake_batch(&IntakeBatchInput {
                id: "batch-interrupted",
                acquisition_channel: IntakeAcquisitionChannel::ExplicitHandoff,
                items: &[IntakeBatchItemInput {
                    id: "item-interrupted",
                    safe_input_label: "statement.pdf",
                    acquisition_input_key: None,
                    acquisition_input_version: None,
                    retry_of_batch_item_id: None,
                }],
            })
            .expect("create pending batch");
    }
    runtime.test_support_lock().expect("lock before restart");
    drop(runtime);

    let restarted = VaultRuntime::with_secret_stores(
        vault_root,
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    restarted
        .unlock(b"synthetic-vault-password")
        .expect("unlock restarted Vault");
    let store = restarted.store().expect("open restarted store");
    let state = store
        .as_ref()
        .expect("unlocked restarted store")
        .intake_item_test_states()
        .expect("read recovered receipt");
    assert_eq!(state.len(), 1);
    assert_eq!(state[0].capture_outcome, "rejected");
    assert_eq!(
        state[0].rejection_kind.as_deref(),
        Some("background_action_required")
    );
    assert_eq!(
        state[0].rejection_code.as_deref(),
        Some("handoff_interrupted")
    );
    assert!(state[0].rejection_resolved_at.is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn local_inbox_failed_capture_retries_the_same_entry_without_erasing_siblings() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    std::fs::create_dir(&inbox_root).expect("create named CanCan Inbox root");
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
        .configure_local_inbox(&inbox_root)
        .expect("authorize named CanCan root");
    let inbox = inbox_root.join("Inbox");
    let failed_path = inbox.join("failed.png");
    let sibling_path = inbox.join("sibling.pdf");
    std::fs::write(&failed_path, b"not a PNG").expect("write invalid image");
    std::fs::write(&sibling_path, b"%PDF-1.4\nsibling statement").expect("write sibling PDF");

    let first = runtime
        .rescan_local_inbox()
        .expect("scan failed and sibling files");
    assert_eq!(first.deferred, 1);
    assert_eq!(first.imported, 1);
    let first_items = {
        let store = runtime.store().expect("open store");
        let store = store.as_ref().expect("unlocked store");
        store
            .intake_item_test_states()
            .expect("read first intake items")
    };
    assert_eq!(first_items.len(), 2);
    let failed_item = first_items
        .iter()
        .find(|item| item.rejection_code.as_deref() == Some("capture_failed"))
        .expect("failed image receipt");
    let failed_item_id = failed_item.id.clone();
    let failed_entry_key = failed_item
        .acquisition_input_key
        .clone()
        .expect("failed image entry key");
    assert_eq!(
        failed_item.rejection_kind.as_deref(),
        Some("background_action_required")
    );
    assert!(
        first_items
            .iter()
            .any(|item| item.capture_outcome == "captured")
    );

    std::fs::write(&failed_path, synthetic_png_fixture()).expect("replace with valid image");
    let second = runtime
        .rescan_local_inbox()
        .expect("retry failed image entry");
    assert_eq!(second.deferred, 0);
    assert_eq!(second.imported, 1);
    let store = runtime.store().expect("reopen store");
    let store = store.as_ref().expect("unlocked store");
    let batches = store
        .intake_batch_test_states()
        .expect("read retry batches");
    assert_eq!(batches.len(), 2);
    assert!(batches.iter().all(|batch| batch.item_count > 0));
    let items = store
        .intake_item_test_states()
        .expect("read retry intake items");
    assert_eq!(items.len(), 3);
    let old_failed = items
        .iter()
        .find(|item| item.id == failed_item_id)
        .expect("old failed receipt");
    assert!(old_failed.rejection_resolved_at.is_some());
    assert_eq!(
        old_failed.resolved_by_batch_item_id.as_deref(),
        items
            .iter()
            .find(|item| {
                item.capture_outcome == "captured"
                    && item.id != failed_item_id
                    && item.acquisition_input_key.as_deref() == Some(failed_entry_key.as_str())
            })
            .map(|item| item.id.as_str())
    );
    assert_eq!(
        items
            .iter()
            .filter(|item| item.capture_outcome == "captured")
            .count(),
        2
    );
}

#[cfg(target_os = "macos")]
#[test]
fn overlapping_local_inbox_scans_seal_one_batch() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    std::fs::create_dir(&inbox_root).expect("create named CanCan Inbox root");
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
        .configure_local_inbox(&inbox_root)
        .expect("authorize named CanCan root");
    std::fs::write(
        inbox_root.join("Inbox").join("overlap.pdf"),
        b"%PDF-1.4\noverlap",
    )
    .expect("write statement");

    let barrier = Arc::new(Barrier::new(3));
    runtime.inject_local_inbox_scan_barrier(barrier.clone());
    let first_runtime = runtime.clone();
    let first = thread::spawn(move || first_runtime.rescan_local_inbox());
    let second_runtime = runtime.clone();
    let second = thread::spawn(move || second_runtime.rescan_local_inbox());
    barrier.wait();
    let first_result = first
        .join()
        .expect("first scan thread")
        .expect("first scan");
    let second_result = second
        .join()
        .expect("second scan thread")
        .expect("second scan");
    runtime.clear_local_inbox_scan_barrier();
    assert_eq!(first_result.imported + second_result.imported, 1);

    let store = runtime.store().expect("open store");
    let store = store.as_ref().expect("unlocked store");
    let batches = store
        .intake_batch_test_states()
        .expect("read overlapping batches");
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].item_count, 1);
}

#[cfg(target_os = "macos")]
#[test]
fn local_inbox_observation_failure_finishes_all_admitted_siblings() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    std::fs::create_dir(&inbox_root).expect("create named CanCan Inbox root");
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
        .configure_local_inbox(&inbox_root)
        .expect("authorize named CanCan root");
    let inbox = inbox_root.join("Inbox");
    std::fs::write(inbox.join("observation-fails.pdf"), b"%PDF-1.4\nfirst")
        .expect("write first statement");
    std::fs::write(inbox.join("sibling.pdf"), b"%PDF-1.4\nsecond")
        .expect("write sibling statement");
    runtime.inject_observation_failure("observation-fails.pdf");
    let error = runtime
        .rescan_local_inbox()
        .expect_err("observation failure should aggregate after the batch");
    assert_eq!(error.code(), "local_inbox_observation_failed");
    let store = runtime.store().expect("open store");
    let store = store.as_ref().expect("unlocked store");
    let items = store.intake_item_test_states().expect("read intake items");
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| item.capture_outcome != "pending"));
    assert_eq!(
        items
            .iter()
            .filter(|item| item.capture_outcome == "captured")
            .count(),
        2
    );
}

#[cfg(target_os = "macos")]
#[test]
fn local_inbox_finalization_failure_recovers_pending_item_and_continues() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    std::fs::create_dir(&inbox_root).expect("create named CanCan Inbox root");
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
        .configure_local_inbox(&inbox_root)
        .expect("authorize named CanCan root");
    let inbox = inbox_root.join("Inbox");
    std::fs::write(inbox.join("failed.png"), b"not a PNG").expect("write invalid image");
    std::fs::write(inbox.join("sibling.pdf"), b"%PDF-1.4\nvalid").expect("write valid sibling");
    {
        let mut store = runtime.store().expect("open store");
        store
            .as_mut()
            .expect("unlocked store")
            .create_intake_batch(&IntakeBatchInput {
                id: "other-local-inbox-batch",
                acquisition_channel: IntakeAcquisitionChannel::LocalInbox,
                items: &[IntakeBatchItemInput {
                    id: "other-local-inbox-item",
                    safe_input_label: "other.pdf",
                    acquisition_input_key: Some(&"d".repeat(64)),
                    acquisition_input_version: Some(&format!("v1:{}", "e".repeat(64))),
                    retry_of_batch_item_id: None,
                }],
            })
            .expect("create unrelated pending batch");
    }
    runtime.inject_intake_finalization_failure();
    let error = runtime
        .rescan_local_inbox()
        .expect_err("finalization failure should be returned");
    assert_eq!(error.code(), "intake_finalization_failed");
    let store = runtime.store().expect("open store");
    let store = store.as_ref().expect("unlocked store");
    let items = store.intake_item_test_states().expect("read intake items");
    assert_eq!(items.len(), 3);
    let unrelated = items
        .iter()
        .find(|item| item.id == "other-local-inbox-item")
        .expect("unrelated pending item");
    assert_eq!(unrelated.capture_outcome, "pending");
    let current_batch_items = items
        .iter()
        .filter(|item| item.id != "other-local-inbox-item")
        .collect::<Vec<_>>();
    assert_eq!(current_batch_items.len(), 2);
    assert!(
        current_batch_items
            .iter()
            .all(|item| item.capture_outcome != "pending")
    );
    assert!(current_batch_items.iter().any(|item| {
        item.capture_outcome == "suppressed"
            && item.rejection_code.as_deref() == Some("discovery_retry")
    }));
    assert!(
        current_batch_items
            .iter()
            .any(|item| item.capture_outcome == "captured")
    );
}
