#[allow(dead_code)]
mod database;
#[allow(dead_code)]
mod vault;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("CanCan desktop runtime failed");
}
