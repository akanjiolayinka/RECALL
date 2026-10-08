mod commands;
mod error;
mod files;
mod locations;
mod scanning;

use std::sync::Mutex;

use locations::LocationStore;
use scanning::ScanStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(LocationStore::default()))
        .manage(ScanStore::default())
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::locations::add_location,
            commands::locations::list_locations,
            commands::locations::remove_location,
            commands::locations::rescan_location,
            commands::files::list_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
