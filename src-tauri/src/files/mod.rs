//! Finding files on disk and describing them.
//!
//! Everything here is plain Rust with no Tauri dependency, so it can be unit
//! tested and reused by the indexing pipeline.

pub mod hash;
pub mod kind;
pub mod scan;

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub use kind::FileKind;

/// Where a file is in Recall's pipeline. Stored in `files.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// Found and fingerprinted; contents not indexed yet.
    Pending,
    /// Contents indexed (from Milestone 5).
    Indexed,
    /// Couldn't be read; see `FileRecord::error`.
    Error,
}

impl FileStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Indexed => "indexed",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "indexed" => Some(Self::Indexed),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// A file stored in the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    pub id: i64,
    pub location_id: i64,
    pub path: PathBuf,
    pub kind: FileKind,
    pub size_bytes: u64,
    /// Milliseconds since the Unix epoch.
    pub modified_at: Option<i64>,
    pub created_at: Option<i64>,
    pub status: FileStatus,
    /// Why the file couldn't be read, for display.
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

/// Milliseconds since the Unix epoch, the time format used everywhere in Recall.
pub fn unix_millis(time: SystemTime) -> Option<i64> {
    i64::try_from(time.duration_since(UNIX_EPOCH).ok()?.as_millis()).ok()
}

pub fn now_millis() -> i64 {
    unix_millis(SystemTime::now()).unwrap_or(0)
}
