mod content_fingerprint;
mod database;
mod diagnostics;
mod local_inbox;
mod phone_shortcut;
#[cfg(test)]
mod presentation_types;
mod runtime;
mod source_file;
mod source_observations;
mod vault;
mod viewer;

#[cfg(target_os = "macos")]
use runtime::setup_background_intake_menu_bar;
use runtime::{
    VaultRuntime, accept_review_relationship, acknowledge_review_item,
    audit_duplicate_committed_versions, choose_local_inbox_root, close_source_document_view,
    confirm_source_candidate, create_money_source, create_vault, decide_candidate_accounts,
    delete_source_document, disable_local_inbox, edit_money_source, edit_review_record,
    enqueue_commit_review_batch, forget_vault_on_this_mac, get_money_overview,
    get_money_source_detail, get_review_detail, get_review_job, import_source_document,
    install_intake_notifications, install_phone_shortcut, intake_notification_settings,
    list_account_confirmation_prompts, list_money_sources, list_recent_activity,
    list_relationship_candidates, list_review_items, list_source_confirmation_prompts,
    list_source_documents, list_statement_password_sources, list_tasks,
    list_unassigned_source_documents, local_inbox_status, lock_vault, on_run_event,
    on_window_event, open_notification_settings, operational_diagnostics_preview,
    park_source_candidate, phone_shortcut_status, preview_source_document,
    remember_vault_on_this_mac, remove_review_record, remove_statement_password,
    render_source_document_page, reparse_source_document, rescan_local_inbox,
    restore_dismissed_candidate_account, save_operational_diagnostics, save_recovery_file,
    save_source_document_copy, set_intake_notifications_enabled, setup_background_window,
    take_background_intake_route, test_phone_shortcut_inbox, try_saved_statement_passwords,
    undo_committed_event, unlock_source_document, unlock_vault, unlock_vault_with_keychain,
    vault_access_status, vault_status,
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
            setup_background_window(app)?;
            #[cfg(target_os = "macos")]
            setup_background_intake_menu_bar(app.handle())?;
            let runtime = app.state::<VaultRuntime>().inner().clone();
            install_intake_notifications(app.handle(), &runtime);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_status,
            vault_access_status,
            audit_duplicate_committed_versions,
            choose_local_inbox_root,
            local_inbox_status,
            disable_local_inbox,
            rescan_local_inbox,
            create_vault,
            unlock_vault,
            unlock_vault_with_keychain,
            remember_vault_on_this_mac,
            forget_vault_on_this_mac,
            remove_statement_password,
            list_statement_password_sources,
            list_tasks,
            intake_notification_settings,
            set_intake_notifications_enabled,
            open_notification_settings,
            take_background_intake_route,
            phone_shortcut_status,
            install_phone_shortcut,
            test_phone_shortcut_inbox,
            try_saved_statement_passwords,
            unlock_source_document,
            lock_vault,
            operational_diagnostics_preview,
            save_operational_diagnostics,
            save_recovery_file,
            save_source_document_copy,
            import_source_document,
            delete_source_document,
            list_money_sources,
            create_money_source,
            edit_money_source,
            get_money_source_detail,
            list_account_confirmation_prompts,
            list_source_confirmation_prompts,
            confirm_source_candidate,
            park_source_candidate,
            decide_candidate_accounts,
            restore_dismissed_candidate_account,
            list_source_documents,
            list_unassigned_source_documents,
            reparse_source_document,
            render_source_document_page,
            preview_source_document,
            close_source_document_view,
            list_review_items,
            get_review_detail,
            list_recent_activity,
            get_money_overview,
            list_relationship_candidates,
            edit_review_record,
            remove_review_record,
            acknowledge_review_item,
            accept_review_relationship,
            enqueue_commit_review_batch,
            get_review_job,
            undo_committed_event
        ])
        .on_window_event(on_window_event)
        .build(tauri::generate_context!())
        .expect("CanCan desktop runtime failed")
        .run(on_run_event);
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
