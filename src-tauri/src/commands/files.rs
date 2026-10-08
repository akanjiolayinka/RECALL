use serde::{Deserialize, Serialize};
use tauri::State;

use crate::database::documents::{self, StoredDocument};
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
    /// "pending" (contents not read yet), "indexed" or "error".
    pub status: &'static str,
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
            status: record.status.as_str(),
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

/// One page (or the whole text, for formats without pages) of a document.
/// Mirrors `DocumentPage` in src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPageDto {
    pub number: Option<u32>,
    pub text: String,
}

/// The text Recall extracted from a file. Mirrors `DocumentText` in src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDto {
    pub file_id: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: Option<u32>,
    pub word_count: u32,
    pub chunk_count: u32,
    pub pages: Vec<DocumentPageDto>,
}

impl DocumentDto {
    fn new(file_id: i64, document: StoredDocument) -> Self {
        Self {
            file_id: file_id.to_string(),
            title: document.title,
            author: document.author,
            page_count: document.page_count,
            word_count: document.word_count,
            chunk_count: document.chunk_count,
            pages: document
                .pages
                .into_iter()
                .map(|page| DocumentPageDto {
                    number: page.number,
                    text: page.text,
                })
                .collect(),
        }
    }
}

/// The text extracted from a file, or `None` if it hasn't been read (yet).
#[tauri::command]
pub fn get_document(
    file_id: String,
    db: State<'_, Database>,
) -> Result<Option<DocumentDto>, ApiError> {
    let id = parse_id(
        &file_id,
        "file_not_found",
        "That file is no longer in your library.",
    )?;
    let document = documents::get_document(&db.connect()?, id)?;
    Ok(document.map(|document| DocumentDto::new(id, document)))
}
