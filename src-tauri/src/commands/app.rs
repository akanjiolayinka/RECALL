use serde::Serialize;

/// Basic information about the running application.
/// Mirrors `AppInfo` in src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    /// Always "tauri" here; the frontend mock reports "mock".
    pub backend: &'static str,
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        backend: "tauri",
    }
}
