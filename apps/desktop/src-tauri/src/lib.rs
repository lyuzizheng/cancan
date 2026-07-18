#[allow(dead_code)]
mod database;
mod runtime;
#[allow(dead_code)]
mod vault;

use runtime::{
    VaultRuntime, create_vault, import_source_document, list_source_documents, lock_vault,
    unlock_vault, vault_status,
};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let root = app.path().app_data_dir()?.join("vault");
            app.manage(VaultRuntime::new(root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_status,
            create_vault,
            unlock_vault,
            lock_vault,
            import_source_document,
            list_source_documents
        ])
        .run(tauri::generate_context!())
        .expect("CanCan desktop runtime failed");
}
