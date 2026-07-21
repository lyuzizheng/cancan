#[allow(dead_code)]
mod database;
mod runtime;
mod system_lock;
#[allow(dead_code)]
mod vault;
mod viewer;

use runtime::{
    VaultRuntime, create_vault, delete_source_document, forget_vault_on_this_mac,
    import_source_document, list_source_documents, list_unassigned_source_documents, lock_vault,
    normalize_source_document, remember_vault_on_this_mac, render_source_document_page,
    save_recovery_file, unlock_vault, unlock_vault_with_keychain, vault_access_status,
    vault_status,
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
            create_vault,
            unlock_vault,
            unlock_vault_with_keychain,
            remember_vault_on_this_mac,
            forget_vault_on_this_mac,
            lock_vault,
            save_recovery_file,
            import_source_document,
            delete_source_document,
            list_source_documents,
            list_unassigned_source_documents,
            normalize_source_document,
            render_source_document_page
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
