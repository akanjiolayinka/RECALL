//! Locations: the folders the user has asked Recall to index.
//!
//! This module holds the rules about which folders may be added. It knows
//! nothing about Tauri, so it can be tested on its own.
//!
//! Locations are currently kept in memory only and are forgotten when the app
//! closes. They move into the SQLite database in Milestone 4.

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub id: String,
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

/// Result of adding a folder.
#[derive(Debug)]
pub struct Added {
    pub location: Location,
    /// Existing locations that were inside the new folder and have been
    /// merged into it, so no file is indexed twice.
    pub replaced: Vec<Location>,
}

#[derive(Debug, Default)]
pub struct LocationStore {
    locations: Vec<Location>,
    next_id: u64,
}

impl LocationStore {
    pub fn list(&self) -> &[Location] {
        &self.locations
    }

    pub fn add(&mut self, path: &Path) -> Result<Added, LocationError> {
        // `dunce` returns normal `C:\...` paths on Windows instead of the
        // `\\?\C:\...` form produced by `std::fs::canonicalize`.
        let path =
            dunce::canonicalize(path).map_err(|_| LocationError::NotFound(path.to_path_buf()))?;
        if !path.is_dir() {
            return Err(LocationError::NotAFolder(path));
        }

        for existing in &self.locations {
            if existing.path == path {
                return Err(LocationError::AlreadyAdded(path));
            }
            // `Path::starts_with` compares whole components, so
            // "/docs-old" is correctly NOT treated as inside "/docs".
            if path.starts_with(&existing.path) {
                return Err(LocationError::InsideExisting {
                    path,
                    parent: existing.path.clone(),
                });
            }
        }

        let (replaced, kept) = std::mem::take(&mut self.locations)
            .into_iter()
            .partition(|existing| existing.path.starts_with(&path));
        self.locations = kept;

        self.next_id += 1;
        let location = Location {
            id: format!("loc-{}", self.next_id),
            path,
        };
        self.locations.push(location.clone());
        Ok(Added { location, replaced })
    }

    pub fn remove(&mut self, id: &str) -> Result<Location, LocationError> {
        let index = self
            .locations
            .iter()
            .position(|location| location.id == id)
            .ok_or_else(|| LocationError::UnknownId(id.to_string()))?;
        Ok(self.locations.remove(index))
    }
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

    #[test]
    fn adds_a_folder_with_canonical_path() {
        let dir = temp_tree();
        let mut store = LocationStore::default();
        let added = store.add(&dir.path().join("docs/../docs")).unwrap();
        assert_eq!(
            added.location.path,
            dunce::canonicalize(dir.path().join("docs")).unwrap()
        );
        assert_eq!(added.location.display_name(), "docs");
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn rejects_missing_paths_and_files() {
        let dir = temp_tree();
        let mut store = LocationStore::default();
        assert!(matches!(
            store.add(&dir.path().join("nope")),
            Err(LocationError::NotFound(_))
        ));
        assert!(matches!(
            store.add(&dir.path().join("file.txt")),
            Err(LocationError::NotAFolder(_))
        ));
    }

    #[test]
    fn rejects_duplicates_and_folders_inside_existing_locations() {
        let dir = temp_tree();
        let mut store = LocationStore::default();
        store.add(&dir.path().join("docs")).unwrap();
        assert!(matches!(
            store.add(&dir.path().join("docs")),
            Err(LocationError::AlreadyAdded(_))
        ));
        assert!(matches!(
            store.add(&dir.path().join("docs/notes")),
            Err(LocationError::InsideExisting { .. })
        ));
        // A sibling whose name merely starts with the same text is allowed.
        store.add(&dir.path().join("docs-old")).unwrap();
    }

    #[test]
    fn adding_a_parent_replaces_locations_inside_it() {
        let dir = temp_tree();
        let mut store = LocationStore::default();
        store.add(&dir.path().join("docs/notes")).unwrap();
        store.add(&dir.path().join("docs-old")).unwrap();
        let added = store.add(&dir.path().join("docs")).unwrap();
        assert_eq!(added.replaced.len(), 1);
        assert_eq!(added.replaced[0].display_name(), "notes");
        let names: Vec<_> = store.list().iter().map(Location::display_name).collect();
        assert_eq!(names, vec!["docs-old", "docs"]);
    }

    #[test]
    fn removes_by_id() {
        let dir = temp_tree();
        let mut store = LocationStore::default();
        let id = store.add(&dir.path().join("docs")).unwrap().location.id;
        store.remove(&id).unwrap();
        assert!(store.list().is_empty());
        assert_eq!(store.remove(&id), Err(LocationError::UnknownId(id)));
    }
}
