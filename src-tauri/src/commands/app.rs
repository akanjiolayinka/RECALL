use serde::Serialize;
use tauri::State;

use crate::embeddings::EmbeddingModel;
use crate::ocr::OcrModel;

/// Basic information about the running application.
/// Mirrors `AppInfo` in src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    /// Always "tauri" here; the frontend mock reports "mock".
    pub backend: &'static str,
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        backend: "tauri",
    }
}

/// Whether one local AI feature is working. Mirrors `AiFeatureStatus` in
/// src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiFeatureStatus {
    pub available: bool,
    /// The model in use, when available.
    pub model: Option<String>,
    /// User-facing reason, when unavailable.
    pub unavailable_reason: Option<String>,
}

/// Status of Recall's local AI features. Mirrors `AiStatus` in src/lib/api/types.ts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatus {
    /// Meaning-based search (local embedding model).
    pub semantic_search: AiFeatureStatus,
    /// Reading text in images (local OCR model).
    pub ocr: AiFeatureStatus,
}

#[tauri::command]
pub fn get_ai_status(embeddings: State<'_, EmbeddingModel>, ocr: State<'_, OcrModel>) -> AiStatus {
    AiStatus {
        semantic_search: AiFeatureStatus {
            available: embeddings.embedder.is_some(),
            model: embeddings
                .embedder
                .as_ref()
                .map(|e| e.model_id().to_string()),
            unavailable_reason: embeddings.unavailable_reason.clone(),
        },
        ocr: AiFeatureStatus {
            available: ocr.engine.is_some(),
            model: ocr
                .engine
                .as_ref()
                .map(|_| "ocrs text-detection + text-recognition".to_string()),
            unavailable_reason: ocr.unavailable_reason.clone(),
        },
    }
}
