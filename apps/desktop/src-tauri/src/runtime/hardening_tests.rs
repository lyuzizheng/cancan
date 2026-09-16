#[cfg(target_os = "macos")]
use super::test_support::statement_password_runtime;
use super::tests::{
    MemoryLocalInboxBookmarkStore, MemoryRememberedKeyStore, MemoryStatementPasswordStore,
};
#[cfg(target_os = "macos")]
use super::tests::{protected_text_pdf, synthetic_pdf};
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

#[test]
fn rejects_an_oversized_document_import_without_reading_it() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let runtime = VaultRuntime::new(parent.path().join("vault"));
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let oversized = parent.path().join("huge-statement.pdf");
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&oversized)
        .expect("open oversized fixture");
    file.set_len(crate::source_file::MAX_SOURCE_FILE_BYTES + 1)
        .expect("grow sparse source");
    drop(file);

    assert_eq!(
        runtime
            .import_selected_document(&oversized)
            .expect_err("reject oversized document")
            .code(),
        "source_file_too_large"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn replaces_the_cached_decryption_only_while_the_vault_session_lives() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("statement.pdf");
    fs::write(&source_path, synthetic_pdf()).expect("write PDF fixture");
    let vault_root = parent.path().join("vault");
    let runtime = VaultRuntime::new(vault_root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("import PDF");

    runtime
        .render_source_document_page(&imported.document_id, 1)
        .expect("render PDF");
    let encrypted_locator = {
        let store = runtime.store().expect("active store");
        store
            .as_ref()
            .expect("unlocked store")
            .list_unassigned_documents()
            .expect("list imported documents")
            .into_iter()
            .find(|document| document.document_id == imported.document_id)
            .and_then(|document| document.encrypted_locator)
            .expect("PDF encrypted locator")
    };
    fs::remove_file(vault_root.join(encrypted_locator)).expect("remove encrypted blob");

    // The decryption is cached for this Vault session, so a repeated render does
    // not need the stored blob again.
    runtime
        .render_source_document_page(&imported.document_id, 1)
        .expect("render from the session cache");

    // Locking drops the cached plaintext; the same render must now reach storage.
    runtime.test_support_lock().expect("lock Vault");
    runtime
        .unlock(b"synthetic-vault-password")
        .expect("unlock Vault");
    assert_eq!(
        runtime
            .render_source_document_page(&imported.document_id, 1)
            .expect_err("reject a render whose stored blob is gone")
            .code(),
        "document_unavailable"
    );
}

#[test]
fn releasing_the_viewer_drops_the_cached_decryption_before_the_vault_locks() {
    let parent = tempfile::tempdir().expect("temporary app data");
    let source_path = parent.path().join("wise-export.csv");
    fs::write(&source_path, b"date,amount\n2026-07-01,10.00\n").expect("write CSV fixture");
    let vault_root = parent.path().join("vault");
    let runtime = VaultRuntime::new(vault_root.clone());
    runtime
        .create(b"synthetic-vault-password")
        .expect("create Vault");
    let imported = runtime
        .import_selected_document(&source_path)
        .expect("import CSV");

    runtime
        .preview_source_document(&imported.document_id)
        .expect("preview CSV");
    let encrypted_locator = {
        let store = runtime.store().expect("active store");
        store
            .as_ref()
            .expect("unlocked store")
            .list_unassigned_documents()
            .expect("list imported documents")
            .into_iter()
            .find(|document| document.document_id == imported.document_id)
            .and_then(|document| document.encrypted_locator)
            .expect("CSV encrypted locator")
    };
    fs::remove_file(vault_root.join(encrypted_locator)).expect("remove encrypted blob");

    // The decryption is still cached for this Vault session, so a repeated
    // preview does not need the stored blob again.
    runtime
        .preview_source_document(&imported.document_id)
        .expect("preview from the session cache");

    // Closing the viewer drops the plaintext buffer while the Vault session
    // that decrypted it stays live.
    runtime
        .close_source_document_view(&imported.document_id)
        .expect("close viewer");
    assert_eq!(
        runtime
            .preview_source_document(&imported.document_id)
            .expect_err("reject a preview after the viewer released the buffer")
            .code(),
        "document_unavailable"
    );

    // Releasing an already dropped buffer, a document that was never cached,
    // or a closed viewer on a locked Vault is a no-op rather than a failure.
    runtime
        .close_source_document_view(&imported.document_id)
        .expect("closing a released viewer stays a no-op");
    runtime
        .close_source_document_view("missing-document")
        .expect("closing an unknown viewer stays a no-op");
    assert_eq!(
        runtime
            .close_source_document_view("")
            .expect_err("reject an empty document id")
            .code(),
        "invalid_document_request"
    );
    runtime.test_support_lock().expect("lock Vault");
    runtime
        .close_source_document_view(&imported.document_id)
        .expect("closing a viewer while the Vault is locked stays a no-op");
}
