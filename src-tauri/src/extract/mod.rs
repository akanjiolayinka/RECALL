//! Extracting plain text from supported documents.
//!
//! Each format has its own small module. All of them return `Extracted`:
//! text split into pages where the format has pages (PDF), or one unnumbered
//! page otherwise (TXT, Markdown, DOCX). No Tauri or database code here.
//!
//! Images are read with OCR (`crate::ocr`) when its models are installed.

mod docx;
mod pdf;
mod text;

use std::path::Path;

use thiserror::Error;

use crate::files::FileKind;
use crate::ocr::Ocr;

/// Files larger than this are skipped rather than loaded into memory.
pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// 1-based page number, or `None` for formats without pages.
    pub number: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extracted {
    pub pages: Vec<Page>,
    pub title: Option<String>,
    pub author: Option<String>,
}

impl Extracted {
    /// Number of real pages, for formats that have them.
    pub fn page_count(&self) -> Option<usize> {
        self.pages
            .iter()
            .any(|p| p.number.is_some())
            .then_some(self.pages.len())
    }

    pub fn word_count(&self) -> usize {
        self.pages
            .iter()
            .map(|p| p.text.split_whitespace().count())
            .sum()
    }

    fn has_text(&self) -> bool {
        self.pages.iter().any(|p| !p.text.trim().is_empty())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtractError {
    #[error("file is larger than the {} MB limit", MAX_FILE_BYTES / 1024 / 1024)]
    TooLarge,
    #[error("could not read the file: {0}")]
    Io(String),
    /// The file is damaged, encrypted or uses an unsupported variant.
    #[error("could not parse the file: {0}")]
    Malformed(String),
    /// Parsed fine but contains no text (e.g. a scanned PDF without a text layer).
    #[error("no text found")]
    NoText,
    #[error("this file type needs OCR")]
    NeedsOcr,
}

impl ExtractError {
    /// Short explanation suitable for showing to the user.
    pub fn user_message(&self, kind: FileKind) -> String {
        match self {
            Self::TooLarge => format!(
                "This file is larger than {} MB, so Recall skipped it.",
                MAX_FILE_BYTES / 1024 / 1024
            ),
            Self::Io(_) => "Recall couldn't open this file.".into(),
            Self::Malformed(_) => match kind {
                FileKind::Pdf => "This PDF couldn't be read. It may be damaged or password-protected.".into(),
                FileKind::Docx => "This Word document couldn't be read. It may be damaged or password-protected.".into(),
                _ => "This file couldn't be read.".into(),
            },
            Self::NoText if kind == FileKind::Pdf => {
                "No text found in this PDF. It may be a scanned document; Recall can't read scanned PDFs yet.".into()
            }
            Self::NoText if kind == FileKind::Image => "No text found in this image.".into(),
            Self::NoText => "This file has no text in it.".into(),
            Self::NeedsOcr => "Images are read with OCR, whose model files aren't installed.".into(),
        }
    }
}

/// Extract the text of a supported file. Images need `ocr`.
pub fn extract(path: &Path, kind: FileKind, ocr: Option<&Ocr>) -> Result<Extracted, ExtractError> {
    if kind == FileKind::Image {
        let extracted = ocr.ok_or(ExtractError::NeedsOcr)?.read_image(path)?;
        return if extracted.has_text() {
            Ok(extracted)
        } else {
            Err(ExtractError::NoText)
        };
    }
    let size = std::fs::metadata(path)
        .map_err(|e| ExtractError::Io(e.to_string()))?
        .len();
    if size > MAX_FILE_BYTES {
        return Err(ExtractError::TooLarge);
    }
    let bytes = std::fs::read(path).map_err(|e| ExtractError::Io(e.to_string()))?;

    let extracted = match kind {
        FileKind::Text => text::extract_plain(&bytes),
        FileKind::Markdown => text::extract_markdown(&bytes),
        FileKind::Pdf => pdf::extract(&bytes)?,
        FileKind::Docx => docx::extract(&bytes)?,
        FileKind::Image => unreachable!("handled above"),
    };
    if !extracted.has_text() {
        return Err(ExtractError::NoText);
    }
    Ok(extracted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// The synthetic files in the repository's test-data/ folder.
    pub fn test_data(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../test-data")
            .join(relative)
    }

    #[test]
    fn extracts_every_supported_test_document() {
        let cases = [
            (
                "notes/Moving out checklist.md",
                FileKind::Markdown,
                "security deposit",
            ),
            (
                "notes/Meeting notes 2026-05-02.txt",
                FileKind::Text,
                "rainwater tank",
            ),
            (
                "notes/Shopping list (UTF-16).txt",
                FileKind::Text,
                "garden gloves",
            ),
            (
                "documents/Q3 Marketing Plan.docx",
                FileKind::Docx,
                "NGN 4,000,000",
            ),
            (
                "pdf/Tenancy Agreement - Flat 4B.pdf",
                FileKind::Pdf,
                "sixty days",
            ),
        ];
        for (file, kind, expected) in cases {
            let extracted = extract(&test_data(file), kind, None).unwrap();
            let all_text: String = extracted.pages.iter().map(|p| p.text.as_str()).collect();
            assert!(
                all_text.contains(expected),
                "{file} should contain {expected:?}"
            );
        }
    }

    #[test]
    fn keeps_pdf_page_numbers() {
        let extracted = extract(
            &test_data("pdf/Project Proposal - Riverside Community Garden.pdf"),
            FileKind::Pdf,
            None,
        )
        .unwrap();
        assert_eq!(extracted.page_count(), Some(4));
        let budget_page = extracted
            .pages
            .iter()
            .find(|p| p.text.contains("NGN 2,500,000"))
            .unwrap();
        assert_eq!(budget_page.number, Some(3));
    }

    #[test]
    fn reports_a_pdf_without_text() {
        assert_eq!(
            extract(
                &test_data("pdf/Scanned letter (no text layer).pdf"),
                FileKind::Pdf,
                None
            ),
            Err(ExtractError::NoText)
        );
    }

    #[test]
    fn reports_damaged_files_instead_of_crashing() {
        let dir = tempfile::TempDir::new().unwrap();
        let pdf = dir.path().join("broken.pdf");
        let docx = dir.path().join("broken.docx");
        std::fs::write(&pdf, b"%PDF-1.4 this is not really a pdf").unwrap();
        std::fs::write(&docx, b"PK not really a zip").unwrap();
        assert!(matches!(
            extract(&pdf, FileKind::Pdf, None),
            Err(ExtractError::Malformed(_))
        ));
        assert!(matches!(
            extract(&docx, FileKind::Docx, None),
            Err(ExtractError::Malformed(_))
        ));
    }

    #[test]
    fn images_wait_for_ocr() {
        assert_eq!(
            extract(Path::new("photo.png"), FileKind::Image, None),
            Err(ExtractError::NeedsOcr)
        );
    }
}
