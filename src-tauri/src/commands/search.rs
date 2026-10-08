use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::database::{self, Database};
use crate::error::{parse_id, ApiError};
use crate::files::FileKind;
use crate::search::{self, SearchResult, SnippetPart};

/// Default and maximum number of results per search.
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 50;

/// Mirrors `SearchRequest` in src/lib/api/types.ts.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetPartDto {
    pub text: String,
    pub highlight: bool,
}

/// One file that matched. Mirrors `SearchResult` in src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    /// Id of the matching passage.
    pub id: String,
    pub file_id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_kind: FileKind,
    pub page: Option<u32>,
    pub snippet: Vec<SnippetPartDto>,
    /// 0–1, relative to the best result of this search.
    pub relevance: f64,
    pub match_reasons: Vec<String>,
}

impl From<SearchResult> for SearchResultDto {
    fn from(result: SearchResult) -> Self {
        let hit = result.hit;
        Self {
            id: hit.chunk_id.to_string(),
            file_id: hit.file_id.to_string(),
            file_name: hit
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            file_path: hit.path.to_string_lossy().into_owned(),
            file_kind: hit.kind,
            page: hit.page_number,
            snippet: hit
                .snippet
                .into_iter()
                .map(|SnippetPart { text, highlight }| SnippetPartDto { text, highlight })
                .collect(),
            relevance: result.relevance,
            match_reasons: result.match_reasons,
        }
    }
}

/// Searches the text of indexed files. Best matches first, one per file.
#[tauri::command]
pub fn search(
    request: SearchRequest,
    db: State<'_, Database>,
) -> Result<Vec<SearchResultDto>, ApiError> {
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let results = search::search(&db.connect()?, &request.query, limit)?;
    Ok(results.into_iter().map(SearchResultDto::from).collect())
}

/// Opens an indexed file in its default app. Takes a file id, not a path, so
/// the UI can only open files that are in the user's library.
#[tauri::command]
pub fn open_file(
    file_id: String,
    app: tauri::AppHandle,
    db: State<'_, Database>,
) -> Result<(), ApiError> {
    let id = parse_id(
        &file_id,
        "file_not_found",
        "That file is no longer in your library.",
    )?;
    let path = database::files::path(&db.connect()?, id)?.ok_or_else(|| {
        ApiError::new("file_not_found", "That file is no longer in your library.")
    })?;
    if !path.exists() {
        return Err(ApiError::new(
            "file_missing",
            "This file has been moved or deleted since Recall last checked. Rescan the folder to update your library.",
        ));
    }
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|err| {
            eprintln!("recall: could not open file {id}: {err}");
            ApiError::new("open_failed", "Recall couldn't open this file. Is there an app on this computer that opens this type of file?")
        })
}
