mod database;
mod local_inbox;
mod runtime;
mod source_observations;
mod system_lock;
mod vault;
mod viewer;

use runtime::{
    VaultRuntime, accept_review_relationship, choose_local_inbox_root, confirm_candidate_accounts,
    create_vault, delete_source_document, disable_local_inbox, edit_review_record,
    enqueue_commit_review_batch, forget_vault_on_this_mac, get_money_overview, get_review_detail,
    get_review_job, import_source_document, list_account_confirmation_prompts, list_money_sources,
    list_recent_activity, list_relationship_candidates, list_review_items, list_source_documents,
    list_statement_coverage_prompts, list_statement_password_sources,
    list_unassigned_source_documents, local_inbox_status, lock_vault, normalize_source_document,
    preview_source_document, record_statement_coverage_decision, remember_vault_on_this_mac,
    remove_review_record, remove_statement_password, render_source_document_page,
    rescan_local_inbox, save_recovery_file, save_source_document_copy,
    try_saved_statement_password, undo_committed_event, unlock_source_document, unlock_vault,
    unlock_vault_with_keychain, vault_access_status, vault_status,
};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io,
    path::Path,
};
use tauri::Manager;

const VAULT_PROCESS_LOCK_FILE: &str = ".vault-process.lock";

#[derive(Debug)]
struct VaultProcessOwnership {
    _lock: File,
}

impl VaultProcessOwnership {
    fn acquire(app_data_dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(app_data_dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .truncate(false)
            .write(true)
            .open(app_data_dir.join(VAULT_PROCESS_LOCK_FILE))?;
        match lock.try_lock() {
            Ok(()) => Ok(Self { _lock: lock }),
            Err(TryLockError::WouldBlock) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "CanCan Vault is already open in another process",
            )),
            Err(TryLockError::Error(error)) => Err(error),
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let ownership = VaultProcessOwnership::acquire(&app_data_dir)?;
            app.manage(ownership);
            app.manage(VaultRuntime::new(app_data_dir.join("vault")));
            system_lock::install(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_status,
            vault_access_status,
            choose_local_inbox_root,
            local_inbox_status,
            disable_local_inbox,
            rescan_local_inbox,
            list_statement_coverage_prompts,
            record_statement_coverage_decision,
            create_vault,
            unlock_vault,
            unlock_vault_with_keychain,
            remember_vault_on_this_mac,
            forget_vault_on_this_mac,
            remove_statement_password,
            list_statement_password_sources,
            try_saved_statement_password,
            unlock_source_document,
            lock_vault,
            save_recovery_file,
            save_source_document_copy,
            import_source_document,
            delete_source_document,
            list_money_sources,
            list_account_confirmation_prompts,
            confirm_candidate_accounts,
            list_source_documents,
            list_unassigned_source_documents,
            normalize_source_document,
            render_source_document_page,
            preview_source_document,
            list_review_items,
            get_review_detail,
            list_recent_activity,
            get_money_overview,
            list_relationship_candidates,
            edit_review_record,
            remove_review_record,
            accept_review_relationship,
            enqueue_commit_review_batch,
            get_review_job,
            undo_committed_event
        ])
        .run(tauri::generate_context!())
        .expect("CanCan desktop runtime failed");
}

#[cfg(test)]
mod tests {
    use super::VaultProcessOwnership;
    use std::io;

    #[test]
    fn holds_exclusive_vault_process_ownership_until_drop() {
        let app_data_dir = tempfile::tempdir().expect("temporary app data");
        let first = VaultProcessOwnership::acquire(app_data_dir.path())
            .expect("first process owns the Vault");

        let error = VaultProcessOwnership::acquire(app_data_dir.path())
            .expect_err("second process must not own the same Vault");
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert!(!app_data_dir.path().join("vault").exists());

        drop(first);
        VaultProcessOwnership::acquire(app_data_dir.path())
            .expect("ownership is released when the process guard drops");
    }
}
