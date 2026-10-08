use std::path::Path;

use serde::Serialize;

/// The file types Recall supports. Deliberately small: reliability over breadth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Pdf,
    Text,
    Markdown,
    Docx,
    Image,
}

impl FileKind {
    /// Name used in the database and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Text => "text",
            Self::Markdown => "markdown",
            Self::Docx => "docx",
            Self::Image => "image",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [
            Self::Pdf,
            Self::Text,
            Self::Markdown,
            Self::Docx,
            Self::Image,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == value)
    }

    /// Decide the kind from the file extension (case-insensitive).
    /// Returns `None` for unsupported files, which the scanner ignores.
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "pdf" => Some(Self::Pdf),
            "txt" => Some(Self::Text),
            "md" | "markdown" => Some(Self::Markdown),
            "docx" => Some(Self::Docx),
            "png" | "jpg" | "jpeg" | "webp" => Some(Self::Image),
            _ => None,
        }
    }
}

/// Standard MIME type for a supported extension (lowercase, without the dot).
pub fn mime_type(extension: &str) -> &'static str {
    match extension {
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_supported_extensions_case_insensitively() {
        assert_eq!(
            FileKind::from_path(Path::new("a/Budget.PDF")),
            Some(FileKind::Pdf)
        );
        assert_eq!(
            FileKind::from_path(Path::new("notes.md")),
            Some(FileKind::Markdown)
        );
        assert_eq!(
            FileKind::from_path(Path::new("shot.JPEG")),
            Some(FileKind::Image)
        );
        assert_eq!(
            FileKind::from_path(Path::new("report.docx")),
            Some(FileKind::Docx)
        );
    }

    #[test]
    fn names_round_trip() {
        for kind in [
            FileKind::Pdf,
            FileKind::Text,
            FileKind::Markdown,
            FileKind::Docx,
            FileKind::Image,
        ] {
            assert_eq!(FileKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(FileKind::parse("exe"), None);
    }

    #[test]
    fn ignores_unsupported_or_missing_extensions() {
        assert_eq!(FileKind::from_path(Path::new("old.doc")), None);
        assert_eq!(FileKind::from_path(Path::new("Makefile")), None);
        assert_eq!(FileKind::from_path(Path::new("archive.zip")), None);
    }
}
