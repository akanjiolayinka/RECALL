//! Locations: the folders the user has asked Recall to index.
//!
//! This module holds the rules about which folders may be added. It knows
//! nothing about Tauri or the database, so the rules can be tested on their
//! own. Storage lives in `database::locations`.

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub id: i64,
    /// Absolute, canonical path (symlinks and `..` resolved).
    pub path: PathBuf,
}

impl Location {
    /// Folder name for display, falling back to the full path for roots like `/`.
    pub fn display_name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.to_string_lossy().into_owned())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LocationError {
    #[error("folder not found: {0}")]
    NotFound(PathBuf),
    #[error("not a folder: {0}")]
    NotAFolder(PathBuf),
    #[error("folder already added: {0}")]
    AlreadyAdded(PathBuf),
    /// The folder is inside a location that is already being indexed.
    #[error("{path} is inside {parent}")]
    InsideExisting { path: PathBuf, parent: PathBuf },
    #[error("unknown location id: {0}")]
    UnknownId(String),
}

/// What should happen when the user adds a folder.
#[derive(Debug, PartialEq, Eq)]
pub struct AddPlan {
    /// Canonical path of the folder to add.
    pub path: PathBuf,
    /// Existing locations inside the new folder. They are merged into it
    /// (removed), so no file is indexed twice.
    pub replaced: Vec<Location>,
}

/// Decide whether `path` can be added alongside `existing` locations.
pub fn plan_add(path: &Path, existing: &[Location]) -> Result<AddPlan, LocationError> {
    // `dunce` returns normal `C:\...` paths on Windows instead of the
    // `\\?\C:\...` form produced by `std::fs::canonicalize`.
    let path =
        dunce::canonicalize(path).map_err(|_| LocationError::NotFound(path.to_path_buf()))?;
    if !path.is_dir() {
        return Err(LocationError::NotAFolder(path));
    }

    for location in existing {
        if location.path == path {
            return Err(LocationError::AlreadyAdded(path));
        }
        // `Path::starts_with` compares whole components, so
        // "/docs-old" is correctly NOT treated as inside "/docs".
        if path.starts_with(&location.path) {
            return Err(LocationError::InsideExisting {
                path,
                parent: location.path.clone(),
            });
        }
    }

    let replaced = existing
        .iter()
        .filter(|location| location.path.starts_with(&path))
        .cloned()
        .collect();
    Ok(AddPlan { path, replaced })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_tree() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("docs/notes")).unwrap();
        fs::create_dir_all(dir.path().join("docs-old")).unwrap();
        fs::write(dir.path().join("file.txt"), "hello").unwrap();
        dir
    }

    fn location(id: i64, path: PathBuf) -> Location {
        Location {
            id,
            path: dunce::canonicalize(path).unwrap(),
        }
    }

    #[test]
    fn plans_a_folder_with_canonical_path() {
        let dir = temp_tree();
        let plan = plan_add(&dir.path().join("docs/../docs"), &[]).unwrap();
        assert_eq!(
            plan.path,
            dunce::canonicalize(dir.path().join("docs")).unwrap()
        );
        assert!(plan.replaced.is_empty());
    }

    #[test]
    fn rejects_missing_paths_and_files() {
        let dir = temp_tree();
        assert!(matches!(
            plan_add(&dir.path().join("nope"), &[]),
            Err(LocationError::NotFound(_))
        ));
        assert!(matches!(
            plan_add(&dir.path().join("file.txt"), &[]),
            Err(LocationError::NotAFolder(_))
        ));
    }

    #[test]
    fn rejects_duplicates_and_folders_inside_existing_locations() {
        let dir = temp_tree();
        let existing = [location(1, dir.path().join("docs"))];
        assert!(matches!(
            plan_add(&dir.path().join("docs"), &existing),
            Err(LocationError::AlreadyAdded(_))
        ));
        assert!(matches!(
            plan_add(&dir.path().join("docs/notes"), &existing),
            Err(LocationError::InsideExisting { .. })
        ));
        // A sibling whose name merely starts with the same text is allowed.
        assert!(plan_add(&dir.path().join("docs-old"), &existing).is_ok());
    }

    #[test]
    fn adding_a_parent_replaces_locations_inside_it() {
        let dir = temp_tree();
        let notes = location(1, dir.path().join("docs/notes"));
        let old = location(2, dir.path().join("docs-old"));
        let plan = plan_add(&dir.path().join("docs"), &[notes.clone(), old]).unwrap();
        assert_eq!(plan.replaced, vec![notes]);
    }

    #[test]
    fn display_name_is_the_folder_name() {
        let dir = temp_tree();
        assert_eq!(location(1, dir.path().join("docs")).display_name(), "docs");
    }
}
