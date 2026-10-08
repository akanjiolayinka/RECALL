use serde::{Deserialize, Serialize};
use tauri::State;

use crate::database::files::{self as file_store, FileQuery};
use crate::database::Database;
use crate::error::{parse_id, ApiError};
use crate::files::{FileKind, FileRecord};

/// Filters for `list_files`. Mirrors `FileListQuery` in src/lib/api/types.ts.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileListQuery {
    pub location_id: Option<String>,
    pub kind: Option<String>,
    pub name_contains: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// A file Recall found. Mirrors `IndexedFile` in src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDto {
    pub id: String,
    pub location_id: String,
    pub name: String,
    pub path: String,
    pub kind: FileKind,
    pub size_bytes: u64,
    /// Milliseconds since the Unix epoch.
    pub modified_at: Option<i64>,
    pub created_at: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileListPage {
    pub files: Vec<FileDto>,
    /// Number of files matching the filters, across all pages.
    pub total: usize,
}

impl From<FileRecord> for FileDto {
    fn from(record: FileRecord) -> Self {
        Self {
            id: record.id.to_string(),
            location_id: record.location_id.to_string(),
            name: record.file_name(),
            path: record.path.to_string_lossy().into_owned(),
            kind: record.kind,
            size_bytes: record.size_bytes,
            modified_at: record.modified_at,
            created_at: record.created_at,
            error: record.error,
        }
    }
}

/// Lists files in the index, with optional filters and paging.
#[tauri::command]
pub fn list_files(
    query: Option<FileListQuery>,
    db: State<'_, Database>,
) -> Result<FileListPage, ApiError> {
    let request = query.unwrap_or_default();
    let location_id = request
        .location_id
        .as_deref()
        .map(|id| {
            parse_id(
                id,
                "location_not_found",
                "That folder is no longer in your library.",
            )
        })
        .transpose()?;
    let kind = request
        .kind
        .as_deref()
        .map(|kind| {
            FileKind::parse(kind).ok_or_else(|| {
                ApiError::new(
                    "invalid_request",
                    format!("Unknown file type filter: {kind}"),
                )
            })
        })
        .transpose()?;

    let query = FileQuery {
        location_id,
        kind,
        name_contains: request.name_contains,
        limit: request.limit.unwrap_or(file_store::DEFAULT_LIMIT),
        offset: request.offset.unwrap_or(0),
    };
    let (files, total) = file_store::query(&db.connect()?, &query)?;
    Ok(FileListPage {
        files: files.into_iter().map(FileDto::from).collect(),
        total,
    })
}
