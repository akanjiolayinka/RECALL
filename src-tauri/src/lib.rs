mod commands;
mod error;
mod locations;

use std::sync::Mutex;

use locations::LocationStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(LocationStore::default()))
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::locations::add_location,
            commands::locations::list_locations,
            commands::locations::remove_location,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
