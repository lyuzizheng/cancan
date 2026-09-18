//! Phone-capture host behavior: installing the Shortcut's destination check,
//! recording it, and capturing through the Inbox it proves.

use super::test_support::{
    MemoryLocalInboxBookmarkStore, MemoryRememberedKeyStore, MemoryStatementPasswordStore,
};
use super::*;
use crate::phone_shortcut::{
    PHONE_SHORTCUT_CHECK_FILE_NAME, PhoneShortcutInboxCheck, PhoneShortcutInboxCheckState,
};
use std::sync::Arc;

fn runtime_with_vault(
    parent: &Path,
    bookmarks: Arc<MemoryLocalInboxBookmarkStore>,
) -> VaultRuntime {
    let runtime = VaultRuntime::with_secret_stores(
        parent.join("vault"),
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        bookmarks,
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
}

#[test]
fn phone_shortcut_status_is_readable_before_an_inbox_exists() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = runtime_with_vault(
        parent.path(),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );

    let status = runtime.phone_shortcut_status();
    assert_eq!(status.version, crate::phone_shortcut::version());
    assert_eq!(status.name, crate::phone_shortcut::name());
    assert_eq!(
        status.inbox_check,
        PhoneShortcutInboxCheck::never_checked(),
        "a setup row can render before the Inbox is configured"
    );
}

#[test]
fn phone_shortcut_inbox_check_records_a_pass_that_survives_restart_and_lock() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    fs::create_dir(&inbox_root).expect("create named CanCan Inbox root");
    let bookmarks = Arc::new(MemoryLocalInboxBookmarkStore::default());
    let runtime = runtime_with_vault(parent.path(), bookmarks.clone());
    runtime
        .configure_local_inbox(&inbox_root)
        .expect("authorize named CanCan root");

    let checked = runtime
        .test_phone_shortcut_inbox()
        .expect("check the Inbox destination");
    assert_eq!(checked.inbox_check, PhoneShortcutInboxCheck::passed());
    assert_eq!(
        fs::read_dir(inbox_root.join("Inbox"))
            .expect("read Inbox")
            .count(),
        0,
        "the check removes the file it created"
    );
    runtime.test_support_lock().expect("lock before restart");
    drop(runtime);

    // The recorded outcome is device-local: a later run reads it without
    // unlocking the Vault, which is what lets the setup row report the last
    // check before anything is unlocked.
    let restarted = VaultRuntime::with_secret_stores(
        parent.path().join("vault"),
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        bookmarks,
    );
    assert_eq!(restarted.status().expect("status"), VaultStatus::Locked);
    assert_eq!(
        restarted.phone_shortcut_status().inbox_check,
        PhoneShortcutInboxCheck::passed()
    );
}

#[test]
fn phone_shortcut_inbox_check_requires_an_unlocked_vault() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    fs::create_dir(&inbox_root).expect("create named CanCan Inbox root");
    let runtime = runtime_with_vault(
        parent.path(),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .configure_local_inbox(&inbox_root)
        .expect("authorize named CanCan root");
    runtime.test_support_lock().expect("lock Vault");

    assert_eq!(
        runtime
            .test_phone_shortcut_inbox()
            .expect_err("a locked Vault cannot reach the Inbox")
            .code(),
        "vault_locked"
    );
}

#[test]
fn phone_shortcut_inbox_check_requires_a_configured_inbox() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = runtime_with_vault(
        parent.path(),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );

    assert_eq!(
        runtime
            .test_phone_shortcut_inbox()
            .expect_err("an Inbox that was never set up cannot be checked")
            .code(),
        "local_inbox_not_configured"
    );
}

#[test]
fn phone_shortcut_inbox_check_reports_a_bookmark_that_needs_authorizing_again() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let bookmarks = Arc::new(MemoryLocalInboxBookmarkStore::default());
    // The bookmark outlives the run that saved it, so it is in the store
    // before this one reads it — the state a Mac is in when the folder behind
    // it is gone.
    bookmarks
        .save(b"a bookmark this Mac never issued")
        .expect("save unusable bookmark");
    let runtime = runtime_with_vault(parent.path(), bookmarks.clone());

    assert_eq!(
        runtime
            .test_phone_shortcut_inbox()
            .expect_err("an unusable bookmark cannot be checked")
            .code(),
        "local_inbox_reauthorization_required"
    );
}

#[test]
fn phone_shortcut_inbox_check_reports_an_occupied_sentinel_without_touching_it() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    fs::create_dir(&inbox_root).expect("create named CanCan Inbox root");
    let runtime = runtime_with_vault(
        parent.path(),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .configure_local_inbox(&inbox_root)
        .expect("authorize named CanCan root");
    let occupied = inbox_root
        .join("Inbox")
        .join(PHONE_SHORTCUT_CHECK_FILE_NAME);
    fs::write(&occupied, b"a file the user put here").expect("write occupied sentinel");

    let checked = runtime
        .test_phone_shortcut_inbox()
        .expect("check the Inbox destination");
    assert_eq!(
        checked.inbox_check.state,
        PhoneShortcutInboxCheckState::Failed
    );
    assert_eq!(
        checked.inbox_check.reason.as_deref(),
        Some("inbox_check_occupied")
    );
    assert_eq!(
        fs::read(&occupied).expect("read occupied sentinel"),
        b"a file the user put here"
    );
    // The failed check is recorded too: the setup row keeps reporting it.
    assert_eq!(
        runtime.phone_shortcut_status().inbox_check,
        checked.inbox_check
    );
    // The file the check refused to touch never becomes evidence: the scan
    // imports nothing, suppresses nothing, and leaves no receipt behind.
    let summary = runtime.rescan_local_inbox().expect("scan local Inbox");
    assert_eq!(summary.imported, 0);
    assert_eq!(summary.suppressed, 0);
    assert_eq!(summary.already_present, 0);
    let guard = runtime.store().expect("open store");
    assert_eq!(
        guard
            .as_ref()
            .expect("unlocked store")
            .intake_batch_test_states()
            .expect("read intake batches")
            .len(),
        0,
        "a file the check left behind must not create an intake receipt"
    );
    drop(guard);
    assert_eq!(
        fs::read(&occupied).expect("read occupied sentinel"),
        b"a file the user put here"
    );
    assert!(occupied.is_file());
}

#[test]
fn a_phone_share_is_captured_once_after_the_inbox_check_and_stays_traceable() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    fs::create_dir(&inbox_root).expect("create named CanCan Inbox root");
    let runtime = runtime_with_vault(
        parent.path(),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .configure_local_inbox(&inbox_root)
        .expect("authorize named CanCan root");
    assert_eq!(
        runtime
            .test_phone_shortcut_inbox()
            .expect("check the Inbox destination")
            .inbox_check,
        PhoneShortcutInboxCheck::passed()
    );

    // What the phone leaves behind: one shared file in the Inbox folder.
    let shared = inbox_root.join("Inbox").join("phone-share-2026-09.pdf");
    let bytes = b"%PDF-1.4\nphone share";
    fs::write(&shared, bytes).expect("write the shared file");
    assert_eq!(
        runtime.rescan_local_inbox().expect("scan local Inbox"),
        LocalInboxScanSummary {
            already_present: 0,
            deferred: 0,
            imported: 1,
            suppressed: 0,
        }
    );
    assert_eq!(fs::read(&shared).expect("read the source"), bytes);

    // The import left one intake receipt for one file: the batch the Review and
    // Tasks surfaces read.
    let guard = runtime.store().expect("open store");
    let store = guard.as_ref().expect("unlocked store");
    let batches = store
        .intake_batch_test_states()
        .expect("read intake batches");
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].item_count, 1);
    let captured = store.intake_item_test_states().expect("read receipt");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].capture_outcome, "captured");
    let captured_document = captured[0]
        .source_document_id
        .clone()
        .expect("the captured file has a source document");
    // The guard holds the store mutex every other command takes, so it is
    // released before the next one runs.
    drop(guard);

    // A scan that follows a capture with nothing new leaves the receipt alone:
    // the entry it already observed is not considered again.
    assert_eq!(
        runtime.rescan_local_inbox().expect("scan again"),
        LocalInboxScanSummary {
            already_present: 0,
            deferred: 0,
            imported: 0,
            suppressed: 0,
        }
    );
    let guard = runtime.store().expect("open store");
    assert_eq!(
        guard
            .as_ref()
            .expect("unlocked store")
            .intake_batch_test_states()
            .expect("read intake batches")
            .len(),
        1
    );
    drop(guard);

    // Sharing the same file again is a new attempt at the same bytes: the scan
    // recognizes them and captures nothing a second time.
    fs::write(&shared, bytes).expect("re-write the shared file");
    let rescan = runtime
        .rescan_local_inbox()
        .expect("scan the re-shared file");
    assert_eq!(rescan.imported, 0);
    assert_eq!(rescan.already_present, 1);
    assert_eq!(fs::read(&shared).expect("read the source"), bytes);
    let guard = runtime.store().expect("open store");
    let store = guard.as_ref().expect("unlocked store");
    let items = store.intake_item_test_states().expect("read receipt");
    assert_eq!(items.len(), 2);
    let reshared = items
        .iter()
        .find(|item| item.capture_outcome == "already_present")
        .expect("the re-share is recorded as already present");
    assert_eq!(
        reshared.source_document_id.as_deref(),
        Some(captured_document.as_str()),
        "both attempts point at the one document the first share captured"
    );
}
