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

impl From<rusqlite::Error> for ApiError {
    /// Database errors are technical; log the detail and show a plain message.
    fn from(err: rusqlite::Error) -> Self {
        eprintln!("recall: database error: {err}");
        ApiError::new(
            "database_error",
            "Recall couldn't read or update its index. Please try again; if this keeps happening, restart Recall.",
        )
    }
}

/// Parse an id received from the frontend. Ids are opaque strings in the API.
pub fn parse_id(id: &str, not_found_code: &'static str, message: &str) -> Result<i64, ApiError> {
    id.parse()
        .map_err(|_| ApiError::new(not_found_code, message))
}
