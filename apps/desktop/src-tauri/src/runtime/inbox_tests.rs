use super::tests::{
    MemoryLocalInboxBookmarkStore, MemoryRememberedKeyStore, MemoryStatementPasswordStore,
};
use super::*;
use crate::local_inbox::INBOX_DIRECTORY_NAME;

fn configured_runtime() -> (tempfile::TempDir, VaultRuntime, PathBuf) {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    fs::create_dir(&inbox_root).expect("create CanCan root");
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
        .expect("configure local Inbox");
    (parent, runtime, inbox_root.join(INBOX_DIRECTORY_NAME))
}

#[test]
fn rejects_watcher_install_after_vault_lock() {
    let (_parent, runtime, inbox) = configured_runtime();
    let generation = runtime
        .inner
        .vault_session_generation
        .load(Ordering::SeqCst);
    let watcher = LocalInboxWatcher::start(&inbox, || {}).expect("create local Inbox watcher");

    runtime.lock().expect("lock Vault");
    let error = runtime
        .install_local_inbox_watcher(generation, &inbox, watcher)
        .expect_err("stale watcher must not survive lock");

    assert_eq!(error.code(), "vault_locked");
    assert!(
        runtime
            .inner
            .local_inbox_watcher
            .lock()
            .expect("watcher slot")
            .is_none()
    );
}

#[test]
fn installs_current_watcher_and_clears_failure() {
    let (_parent, runtime, inbox) = configured_runtime();
    runtime
        .inner
        .local_inbox_watch_failed
        .store(true, Ordering::SeqCst);
    let generation = runtime
        .inner
        .vault_session_generation
        .load(Ordering::SeqCst);
    let watcher = LocalInboxWatcher::start(&inbox, || {}).expect("create local Inbox watcher");

    runtime
        .install_local_inbox_watcher(generation, &inbox, watcher)
        .expect("install current watcher");

    assert!(
        runtime
            .inner
            .local_inbox_watcher
            .lock()
            .expect("watcher slot")
            .is_some()
    );
    assert!(
        !runtime
            .inner
            .local_inbox_watch_failed
            .load(Ordering::SeqCst)
    );
    assert_eq!(
        runtime
            .local_inbox_status()
            .expect("enabled Inbox status")
            .access_state,
        LocalInboxAccessState::Enabled
    );
}

#[test]
fn watcher_failure_is_sanitized_and_cleared_by_lock() {
    let (_parent, runtime, _inbox) = configured_runtime();
    runtime
        .inner
        .local_inbox_watch_failed
        .store(true, Ordering::SeqCst);

    assert_eq!(
        runtime
            .local_inbox_status()
            .expect_err("watch failure must be actionable")
            .code(),
        "local_inbox_watch_failed"
    );

    runtime.lock().expect("lock Vault");
    assert_eq!(
        runtime
            .local_inbox_status()
            .expect("locked Inbox status")
            .access_state,
        LocalInboxAccessState::Paused
    );
    assert!(
        !runtime
            .inner
            .local_inbox_watch_failed
            .load(Ordering::SeqCst)
    );
}
