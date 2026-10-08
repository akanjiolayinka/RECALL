use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::database::{self, Database};
use crate::embeddings::EmbeddingModel;
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
    /// Unique per result: the passage id, or the file id when only the file
    /// name matched.
    pub id: String,
    pub file_id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_kind: FileKind,
    pub page: Option<u32>,
    /// Empty when only the file name or title matched.
    pub snippet: Vec<SnippetPartDto>,
    /// Combined 0–1 score.
    pub relevance: f64,
    pub match_reasons: Vec<String>,
}

impl From<SearchResult> for SearchResultDto {
    fn from(result: SearchResult) -> Self {
        let (id, page, snippet) = match result.passage {
            Some(passage) => (
                format!("chunk-{}", passage.chunk_id),
                passage.page_number,
                passage
                    .snippet
                    .into_iter()
                    .map(|SnippetPart { text, highlight }| SnippetPartDto { text, highlight })
                    .collect(),
            ),
            None => (format!("file-{}", result.file_id), None, Vec::new()),
        };
        Self {
            id,
            file_id: result.file_id.to_string(),
            file_name: result
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            file_path: result.path.to_string_lossy().into_owned(),
            file_kind: result.kind,
            page,
            snippet,
            relevance: result.relevance,
            match_reasons: result.match_reasons,
        }
    }
}

/// Searches indexed files by meaning (when a model is installed), keywords
/// and file names. Best matches first, one per file.
#[tauri::command]
pub fn search(
    request: SearchRequest,
    db: State<'_, Database>,
    model: State<'_, EmbeddingModel>,
) -> Result<Vec<SearchResultDto>, ApiError> {
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let results = search::search(
        &db.connect()?,
        model.embedder.as_deref(),
        &request.query,
        limit,
    )?;
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

/// Which search strategies are available. Mirrors `SearchCapabilities` in
/// src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCapabilities {
    pub semantic_search: bool,
    /// Embedding model in use, when semantic search is available.
    pub embedding_model: Option<String>,
    /// User-facing reason semantic search is unavailable.
    pub semantic_unavailable_reason: Option<String>,
}

#[tauri::command]
pub fn get_search_capabilities(model: State<'_, EmbeddingModel>) -> SearchCapabilities {
    SearchCapabilities {
        semantic_search: model.embedder.is_some(),
        embedding_model: model.embedder.as_ref().map(|e| e.model_id().to_string()),
        semantic_unavailable_reason: model.unavailable_reason.clone(),
    }
}
