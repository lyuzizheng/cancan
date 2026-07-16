pub mod database;
pub mod vault;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("CanCan desktop runtime failed");
}
