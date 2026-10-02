use std::sync::Mutex;

use serde::Serialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::error::ApiError;
use crate::locations::{Location, LocationError, LocationStore};

/// A folder Recall indexes. Mirrors `Location` in src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationDto {
    pub id: String,
    pub name: String,
    pub path: String,
}

impl From<&Location> for LocationDto {
    fn from(location: &Location) -> Self {
        Self {
            id: location.id.clone(),
            name: location.display_name(),
            path: location.path.to_string_lossy().into_owned(),
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

fn lock(
    store: &Mutex<LocationStore>,
) -> Result<std::sync::MutexGuard<'_, LocationStore>, ApiError> {
    store
        .lock()
        .map_err(|_| ApiError::new("internal", "Something went wrong. Please restart Recall."))
}

/// Opens the system folder picker and adds the chosen folder.
/// Returns `None` if the user closes the picker without choosing.
///
/// The picker runs in Rust (not in the webview) so the backend only ever
/// receives folders the user chose in the system dialog.
#[tauri::command]
pub async fn add_location(
    app: tauri::AppHandle,
    store: State<'_, Mutex<LocationStore>>,
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

    let added = lock(&store)?.add(&path)?;
    Ok(Some(AddLocationResponse {
        location: LocationDto::from(&added.location),
        replaced: added.replaced.iter().map(LocationDto::from).collect(),
    }))
}

#[tauri::command]
pub fn list_locations(
    store: State<'_, Mutex<LocationStore>>,
) -> Result<Vec<LocationDto>, ApiError> {
    Ok(lock(&store)?.list().iter().map(LocationDto::from).collect())
}

/// Removes a folder from the library. Never touches the files themselves.
#[tauri::command]
pub fn remove_location(id: String, store: State<'_, Mutex<LocationStore>>) -> Result<(), ApiError> {
    lock(&store)?.remove(&id)?;
    Ok(())
}
