#[cfg(target_os = "macos")]
use super::test_support::statement_password_runtime;
#[cfg(target_os = "macos")]
use super::tests::protected_text_pdf;
use super::tests::{
    MemoryLocalInboxBookmarkStore, MemoryRememberedKeyStore, MemoryStatementPasswordStore,
};
use super::*;
use std::{fs, path::Path, sync::Arc};

fn test_runtime(root: &Path, bookmarks: Arc<MemoryLocalInboxBookmarkStore>) -> VaultRuntime {
    VaultRuntime::with_secret_stores(
        root.to_path_buf(),
        Arc::new(MemoryRememberedKeyStore::default()),
        Arc::new(MemoryStatementPasswordStore::default()),
        bookmarks,
    )
}

#[test]
fn lock_and_unlock_reject_a_parse_result_extracted_by_the_old_vault_session() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = test_runtime(
        &parent.path().join("vault"),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let source = parent.path().join("statement.csv");
    fs::write(&source, "date,amount\n2026-07-01,10.00\n").expect("write fixture");
    let imported = runtime
        .import_selected_document(&source)
        .expect("import document");
    let job = runtime
        .queued_local_inbox_parse_documents()
        .expect("read parse job")
        .pop()
        .expect("queued parse job");
    let attempt = runtime
        .start_local_inbox_parse(&job)
        .expect("claim parse job")
        .expect("parse job claimed");
    let input = runtime
        .normalization_input(&imported.document_id)
        .expect("extract input in current session");

    runtime.test_support_lock().expect("lock Vault");
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("unlock Vault");
    assert_eq!(
        runtime
            .apply_normalizer_result_for_job(
                &attempt.claim,
                &input,
                NormalizerResult::NeedsAttention {
                    reason: "unsupported_document".to_owned(),
                },
            )
            .expect_err("old extraction must not finalize after a new session")
            .code(),
        "vault_locked"
    );
}

#[test]
fn reopen_recovers_expired_parse_work_when_local_inbox_is_disabled() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let vault_root = parent.path().join("vault");
    let bookmarks = Arc::new(MemoryLocalInboxBookmarkStore::default());
    let runtime = test_runtime(&vault_root, bookmarks.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let source = parent.path().join("statement.csv");
    fs::write(&source, "date,amount\n2026-07-01,10.00\n").expect("write fixture");
    let imported = runtime
        .import_selected_document(&source)
        .expect("import document");
    let job = runtime
        .queued_local_inbox_parse_documents()
        .expect("read parse job")
        .pop()
        .expect("queued parse job");
    let attempt = runtime
        .start_local_inbox_parse(&job)
        .expect("claim parse job")
        .expect("parse job claimed");
    runtime
        .store()
        .expect("open store")
        .as_ref()
        .expect("unlocked store")
        .expire_parse_document_lease_for_test(&attempt.claim.job_id)
        .expect("expire parse lease");
    runtime.test_support_lock().expect("lock Vault");
    drop(runtime);

    let reopened = test_runtime(&vault_root, bookmarks);
    reopened
        .unlock(b"synthetic-vault-password")
        .expect("unlock reopened Vault");
    assert_eq!(
        reopened
            .local_inbox_status()
            .expect("read disabled Inbox state")
            .access_state,
        LocalInboxAccessState::Disabled
    );
    assert_eq!(
        reopened
            .queued_local_inbox_parse_documents()
            .expect("recover durable parse job")
            .into_iter()
            .map(|job| job.document_id)
            .collect::<Vec<_>>(),
        vec![imported.document_id]
    );
}

#[test]
fn reopen_requeues_an_unexpired_parse_claim_and_rejects_the_old_token() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let vault_root = parent.path().join("vault");
    let bookmarks = Arc::new(MemoryLocalInboxBookmarkStore::default());
    let runtime = test_runtime(&vault_root, bookmarks.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let source = parent.path().join("statement.csv");
    fs::write(&source, "date,amount\n2026-07-01,10.00\n").expect("write fixture");
    let imported = runtime
        .import_selected_document(&source)
        .expect("import document");
    let job = runtime
        .queued_local_inbox_parse_documents()
        .expect("read parse job")
        .pop()
        .expect("queued parse job");
    let first = runtime
        .start_local_inbox_parse(&job)
        .expect("claim parse job")
        .expect("parse job claimed");
    runtime.test_support_lock().expect("lock Vault");
    drop(runtime);

    let reopened = test_runtime(&vault_root, bookmarks);
    reopened
        .unlock(b"synthetic-vault-password")
        .expect("unlock reopened Vault");
    let requeued = reopened
        .queued_local_inbox_parse_documents()
        .expect("recover unexpired parse job")
        .pop()
        .expect("unexpired parse job is requeued");
    assert_eq!(requeued.document_id, imported.document_id);
    assert_eq!(requeued.job_id, job.job_id);
    assert_eq!(requeued.logical_run_key, job.logical_run_key);
    let second = reopened
        .start_local_inbox_parse(&requeued)
        .expect("claim requeued parse job")
        .expect("requeued parse job claimed");
    assert_ne!(first.claim.claim_token, second.claim.claim_token);
    assert!(
        reopened
            .store()
            .expect("open store")
            .as_mut()
            .expect("unlocked store")
            .block_parse_document_job(&first.claim, "password_required")
            .is_err()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn background_inbox_failure_changes_enabled_status_to_needs_attention() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let inbox_root = parent.path().join("Cancan");
    fs::create_dir(&inbox_root).expect("create named CanCan root");
    let runtime = test_runtime(
        &parent.path().join("vault"),
        Arc::new(MemoryLocalInboxBookmarkStore::default()),
    );
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    runtime
        .configure_local_inbox(&inbox_root)
        .expect("configure local Inbox");
    assert_eq!(
        runtime
            .local_inbox_status()
            .expect("read enabled Inbox")
            .access_state,
        LocalInboxAccessState::Enabled
    );

    runtime.mark_local_inbox_needs_attention();

    assert_eq!(
        runtime
            .local_inbox_status()
            .expect("read failed Inbox")
            .access_state,
        LocalInboxAccessState::NeedsAttention
    );

    runtime
        .rescan_local_inbox()
        .expect("successful retry clears background failure");
    assert_eq!(
        runtime
            .local_inbox_status()
            .expect("read recovered Inbox")
            .access_state,
        LocalInboxAccessState::Enabled
    );
}

#[cfg(target_os = "macos")]
#[test]
fn unlocking_a_password_blocked_parse_requeues_the_same_logical_run() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let statement_passwords = Arc::new(MemoryStatementPasswordStore::default());
    let runtime =
        statement_password_runtime(&parent.path().join("vault"), statement_passwords.clone());
    let source = parent.path().join("protected-statement.pdf");
    fs::write(&source, protected_text_pdf()).expect("write protected PDF fixture");
    let imported = runtime
        .import_selected_document(&source)
        .expect("import protected statement");
    let job = runtime
        .queued_local_inbox_parse_documents()
        .expect("read parse job")
        .pop()
        .expect("queued parse job");
    let attempt = runtime
        .start_local_inbox_parse(&job)
        .expect("claim parse job")
        .expect("parse job claimed");
    assert_eq!(
        runtime
            .normalization_input(&imported.document_id)
            .expect_err("protected PDF blocks parsing without a password")
            .code(),
        "statement_password_required"
    );
    runtime
        .block_local_inbox_parse(&attempt, "password_required")
        .expect("block parse pending password");
    assert_eq!(
        runtime
            .list_unassigned_source_documents()
            .expect("read blocked document")[0]
            .document_status,
        SourceDocumentStatus::NeedsAttention
    );

    runtime
        .unlock_source_document(
            &imported.document_id,
            "source-dbs",
            b"statement-password",
            true,
        )
        .expect("verify cache and save password");

    assert_eq!(
        statement_passwords
            .load("money-source:source-dbs")
            .expect("load saved password")
            .expect("saved password")
            .as_slice(),
        b"statement-password"
    );
    let requeued = runtime
        .queued_local_inbox_parse_documents()
        .expect("read requeued parse job")
        .pop()
        .expect("same logical run is queued");
    assert_eq!(requeued.job_id, job.job_id);
    assert_eq!(requeued.logical_run_key, job.logical_run_key);
    assert_eq!(
        runtime
            .list_unassigned_source_documents()
            .expect("read processing document")[0]
            .document_status,
        SourceDocumentStatus::Processing
    );
    assert!(runtime.normalization_input(&imported.document_id).is_ok());
}
