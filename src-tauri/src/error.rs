use serde::Serialize;

/// The error shape every Tauri command returns to the frontend.
/// Mirrors `ApiError` in src/lib/api/types.ts and is documented in docs/API.md.
///
/// `code` is stable and machine-readable; `message` is written for users.
/// Technical detail belongs in logs, not in `message`.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
