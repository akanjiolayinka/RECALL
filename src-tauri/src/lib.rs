mod commands;
mod database;
mod error;
mod extract;
mod files;
mod indexing;
mod locations;
mod scanning;
mod search;

use tauri::Manager;

use database::Database;
use scanning::{start_scan, ScanStore};

/// Opens the database in the app data folder and re-checks every saved
/// folder, so changes made while Recall was closed are picked up.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    let db = Database::open(&data_dir.join(database::DATABASE_FILE_NAME))?;

    let locations = database::locations::list(&db.connect()?)?;
    app.manage(db);
    for location in locations {
        start_scan(app.handle(), location.id, location.path);
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(ScanStore::default())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::locations::add_location,
            commands::locations::list_locations,
            commands::locations::remove_location,
            commands::locations::rescan_location,
            commands::files::list_files,
            commands::files::get_document,
            commands::search::search,
            commands::search::open_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
