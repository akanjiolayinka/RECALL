use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::ApiError;
use crate::files::query::{self, FileQuery};
use crate::files::{FileKind, FileRecord};
use crate::scanning::ScanStore;

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

fn to_millis(time: Option<std::time::SystemTime>) -> Option<i64> {
    let millis = time?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    i64::try_from(millis).ok()
}

impl From<&FileRecord> for FileDto {
    fn from(record: &FileRecord) -> Self {
        Self {
            id: record.id.clone(),
            location_id: record.location_id.clone(),
            name: record.file_name(),
            path: record.path.to_string_lossy().into_owned(),
            kind: record.kind,
            size_bytes: record.size_bytes,
            modified_at: to_millis(record.modified_at),
            created_at: to_millis(record.created_at),
            error: record.error.clone(),
        }
    }
}

fn parse_kind(kind: &str) -> Result<FileKind, ApiError> {
    match kind {
        "pdf" => Ok(FileKind::Pdf),
        "text" => Ok(FileKind::Text),
        "markdown" => Ok(FileKind::Markdown),
        "docx" => Ok(FileKind::Docx),
        "image" => Ok(FileKind::Image),
        _ => Err(ApiError::new(
            "invalid_request",
            format!("Unknown file type filter: {kind}"),
        )),
    }
}

/// Lists files found in the library, with optional filters and paging.
#[tauri::command]
pub fn list_files(
    query: Option<FileListQuery>,
    scans: State<'_, ScanStore>,
) -> Result<FileListPage, ApiError> {
    let request = query.unwrap_or_default();
    let query = FileQuery {
        location_id: request.location_id,
        kind: request.kind.as_deref().map(parse_kind).transpose()?,
        name_contains: request.name_contains,
        limit: request.limit.unwrap_or(query::DEFAULT_LIMIT),
        offset: request.offset.unwrap_or(0),
    };
    Ok(scans.with_files(|records| {
        let (page, total) = query::apply(records, &query);
        FileListPage {
            files: page.into_iter().map(FileDto::from).collect(),
            total,
        }
    }))
}
