//! Where Recall looks for local AI model files.
//!
//! Model files are not stored in Git. `npm run download-models` puts them in
//! `models/` at the repository root (see docs/MODELS.md).

use std::path::PathBuf;

use tauri::Manager;

/// Set this environment variable to use models from another folder.
pub const MODELS_DIR_ENV: &str = "RECALL_MODELS_DIR";

/// Folders that may contain models, most specific first.
pub fn candidate_dirs(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os(MODELS_DIR_ENV) {
        dirs.push(PathBuf::from(dir));
    }
    // Models bundled with an installed app (Milestone 17).
    if let Ok(resources) = app.path().resource_dir() {
        dirs.push(resources.join("models"));
    }
    // During development: <repository>/models.
    if cfg!(debug_assertions) {
        dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models"));
    }
    dirs
}
