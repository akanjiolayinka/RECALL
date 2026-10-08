//! PDF text, page by page, using the pure-Rust `pdf-extract` crate.

use std::panic::{catch_unwind, AssertUnwindSafe};

use super::{ExtractError, Extracted, Page};

pub fn extract(bytes: &[u8]) -> Result<Extracted, ExtractError> {
    // pdf-extract can panic on unusual or malformed PDFs. Contain that here so
    // one bad file is reported as unreadable instead of stopping Recall.
    let pages = catch_unwind(AssertUnwindSafe(|| {
        pdf_extract::extract_text_from_mem_by_pages(bytes)
    }))
    .map_err(|_| ExtractError::Malformed("PDF parser panicked".into()))?
    .map_err(|err| ExtractError::Malformed(err.to_string()))?;

    let pages = pages
        .into_iter()
        .enumerate()
        .map(|(index, text)| Page {
            number: u32::try_from(index + 1).ok(),
            text: tidy(&text),
        })
        .collect();
    Ok(Extracted {
        pages,
        title: None,
        author: None,
    })
}

/// pdf-extract keeps the layout's blank lines; collapse runs of them so text
/// reads naturally, and trim trailing spaces from each line.
fn tidy(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidy_collapses_blank_lines() {
        assert_eq!(tidy("a  \n\n\n\nb\n\n"), "a\n\nb");
    }
}
