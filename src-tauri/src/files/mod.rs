//! Finding files on disk and describing them.
//!
//! Everything here is plain Rust with no Tauri dependency, so it can be unit
//! tested and reused by the indexing pipeline.

pub mod hash;
pub mod kind;
pub mod query;
pub mod scan;

use std::path::PathBuf;
use std::time::SystemTime;

pub use kind::FileKind;

/// A supported file found inside a location, plus what we learned about it.
#[derive(Debug, Clone)]
pub struct FileRecord {
    pub id: String,
    pub location_id: String,
    pub path: PathBuf,
    pub kind: FileKind,
    pub size_bytes: u64,
    pub modified_at: Option<SystemTime>,
    pub created_at: Option<SystemTime>,
    /// SHA-256 of the file contents, hex encoded. `None` if it couldn't be read.
    #[expect(
        dead_code,
        reason = "computed now; used for change detection once files are stored in SQLite (Milestone 4)"
    )]
    pub content_hash: Option<String>,
    /// Why the file couldn't be read, for display. `None` when all went well.
    pub error: Option<String>,
}

impl FileRecord {
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
