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
    unlock_vault, unlock_vault_with_keychain, vault_access_status, vault_status,
};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let root = app.path().app_data_dir()?.join("vault");
            app.manage(VaultRuntime::new(root));
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
