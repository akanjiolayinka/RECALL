use serde::Serialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::database::{self, Database};
use crate::error::{parse_id, ApiError};
use crate::files::now_millis;
use crate::locations::{plan_add, Location, LocationError};
use crate::scanning::{start_scan, ScanStatus, ScanStore};
use crate::watching::FolderWatcher;

/// A folder Recall indexes. Mirrors `Location` in src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationDto {
    pub id: String,
    pub name: String,
    pub path: String,
    /// Files from this folder currently in the index.
    pub file_count: usize,
    /// Latest scan progress since Recall started, or `None` if not scanned yet.
    pub scan: Option<ScanStatus>,
}

impl LocationDto {
    fn new(location: &Location, file_count: usize, scans: &ScanStore) -> Self {
        Self {
            id: location.id.to_string(),
            name: location.display_name(),
            path: location.path.to_string_lossy().into_owned(),
            file_count,
            scan: scans.status(location.id),
        }
    }
}

/// Response for `add_location`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddLocationResponse {
    pub location: LocationDto,
    /// Previously added folders that were inside the new one and merged into it.
    pub replaced: Vec<LocationDto>,
}

impl From<LocationError> for ApiError {
    fn from(err: LocationError) -> Self {
        match err {
            LocationError::NotFound(path) => ApiError::new(
                "folder_not_found",
                format!("We couldn't find the folder {}.", path.display()),
            ),
            LocationError::NotAFolder(path) => ApiError::new(
                "not_a_folder",
                format!("{} is a file, not a folder.", path.display()),
            ),
            LocationError::AlreadyAdded(path) => ApiError::new(
                "already_added",
                format!("{} is already in your library.", path.display()),
            ),
            LocationError::InsideExisting { path, parent } => ApiError::new(
                "already_covered",
                format!(
                    "{} is already included, because it's inside {}.",
                    path.display(),
                    parent.display()
                ),
            ),
            LocationError::UnknownId(_) => ApiError::new(
                "location_not_found",
                "That folder is no longer in your library.",
            ),
        }
    }
}

fn parse_location_id(id: &str) -> Result<i64, ApiError> {
    parse_id(
        id,
        "location_not_found",
        "That folder is no longer in your library.",
    )
}

/// Opens the system folder picker, adds the chosen folder and starts scanning it.
/// Returns `None` if the user closes the picker without choosing.
///
/// The picker runs in Rust (not in the webview) so the backend only ever
/// receives folders the user chose in the system dialog.
#[tauri::command]
pub async fn add_location(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    scans: State<'_, ScanStore>,
    watcher: State<'_, FolderWatcher>,
) -> Result<Option<AddLocationResponse>, ApiError> {
    // `blocking_pick_folder` must not run on the main thread; async commands
    // run on a background thread, as in the plugin's documented example.
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Choose a folder for Recall to index")
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|_| {
        ApiError::new(
            "unsupported_location",
            "That location can't be indexed. Please choose a folder on this computer.",
        )
    })?;

    let mut conn = db.connect()?;
    let plan = plan_add(&path, &database::locations::list(&conn)?)?;
    let tx = conn.transaction()?;
    for replaced in &plan.replaced {
        database::locations::delete(&tx, replaced.id)?;
    }
    let location = database::locations::insert(&tx, &plan.path, now_millis())?;
    tx.commit()?;

    for replaced in &plan.replaced {
        scans.forget(replaced.id);
        watcher.unwatch(&replaced.path);
    }
    watcher.watch(&location.path);
    start_scan(&app, location.id, location.path.clone());

    Ok(Some(AddLocationResponse {
        location: LocationDto::new(&location, 0, &scans),
        replaced: plan
            .replaced
            .iter()
            .map(|l| LocationDto::new(l, 0, &scans))
            .collect(),
    }))
}

#[tauri::command]
pub fn list_locations(
    db: State<'_, Database>,
    scans: State<'_, ScanStore>,
) -> Result<Vec<LocationDto>, ApiError> {
    let conn = db.connect()?;
    let counts = database::files::count_by_location(&conn)?;
    Ok(database::locations::list(&conn)?
        .iter()
        .map(|l| LocationDto::new(l, counts.get(&l.id).copied().unwrap_or(0), &scans))
        .collect())
}

/// Removes a folder from the library and everything indexed from it.
/// Never touches the files themselves.
#[tauri::command]
pub fn remove_location(
    id: String,
    db: State<'_, Database>,
    scans: State<'_, ScanStore>,
    watcher: State<'_, FolderWatcher>,
) -> Result<(), ApiError> {
    let id = parse_location_id(&id)?;
    let conn = db.connect()?;
    let location = database::locations::get(&conn, id)?
        .ok_or_else(|| ApiError::from(LocationError::UnknownId(id.to_string())))?;
    // Stop watching and scanning first so nothing keeps writing rows for it.
    watcher.unwatch(&location.path);
    scans.forget(id);
    database::locations::delete(&conn, id)?;
    Ok(())
}

/// Checks a folder again for new, changed and deleted files.
#[tauri::command]
pub fn rescan_location(
    id: String,
    app: tauri::AppHandle,
    db: State<'_, Database>,
) -> Result<(), ApiError> {
    let id = parse_location_id(&id)?;
    let location = database::locations::get(&db.connect()?, id)?
        .ok_or_else(|| ApiError::from(LocationError::UnknownId(id.to_string())))?;
    start_scan(&app, location.id, location.path);
    Ok(())
}
