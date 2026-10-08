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
    fn ignores_unsupported_or_missing_extensions() {
        assert_eq!(FileKind::from_path(Path::new("old.doc")), None);
        assert_eq!(FileKind::from_path(Path::new("Makefile")), None);
        assert_eq!(FileKind::from_path(Path::new("archive.zip")), None);
    }
}
