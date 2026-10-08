//! Word (.docx) text.
//!
//! A .docx file is a ZIP archive. The body text is in `word/document.xml`
//! (text runs in `<w:t>` elements, paragraphs in `<w:p>`); the title and
//! author are in `docProps/core.xml`. Word doesn't store page breaks reliably,
//! so the whole document is one unnumbered page.

use std::io::{Cursor, Read};

use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesRef, Event};
use quick_xml::Reader;
use zip::ZipArchive;

use super::{ExtractError, Extracted, Page};

/// Refuse absurdly large XML parts, e.g. a "zip bomb".
const MAX_PART_BYTES: u64 = 200 * 1024 * 1024;

pub fn extract(bytes: &[u8]) -> Result<Extracted, ExtractError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| ExtractError::Malformed(e.to_string()))?;
    let document = read_part(&mut archive, "word/document.xml")?
        .ok_or_else(|| ExtractError::Malformed("missing word/document.xml".into()))?;
    let text = body_text(&document)?;

    let (title, author) = match read_part(&mut archive, "docProps/core.xml")? {
        Some(core) => core_properties(&core),
        None => (None, None),
    };
    Ok(Extracted {
        pages: vec![Page { number: None, text }],
        title,
        author,
    })
}

fn read_part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<String>, ExtractError> {
    let file = match archive.by_name(name) {
        Ok(file) => file,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(ExtractError::Malformed(e.to_string())),
    };
    let mut xml = String::new();
    file.take(MAX_PART_BYTES)
        .read_to_string(&mut xml)
        .map_err(|e| ExtractError::Malformed(e.to_string()))?;
    Ok(Some(xml))
}

/// The character an `&...;` reference stands for, e.g. `&amp;` → `&`.
fn resolve_reference(reference: &BytesRef<'_>) -> Option<String> {
    if let Ok(Some(c)) = reference.resolve_char_ref() {
        return Some(c.to_string());
    }
    let name = reference.decode().ok()?;
    resolve_predefined_entity(&name).map(str::to_string)
}

fn body_text(xml: &str) -> Result<String, ExtractError> {
    let mut reader = Reader::from_str(xml);
    let mut text = String::new();
    let mut in_text_run = false;

    loop {
        match reader
            .read_event()
            .map_err(|e| ExtractError::Malformed(e.to_string()))?
        {
            Event::Start(e) if e.local_name().as_ref() == b"t" => in_text_run = true,
            Event::End(e) => match e.local_name().as_ref() {
                b"t" => in_text_run = false,
                b"p" => text.push('\n'),
                _ => {}
            },
            Event::Empty(e) => match e.local_name().as_ref() {
                b"tab" => text.push('\t'),
                b"br" | b"cr" => text.push('\n'),
                _ => {}
            },
            Event::Text(e) if in_text_run => {
                text.push_str(
                    &e.decode()
                        .map_err(|e| ExtractError::Malformed(e.to_string()))?,
                );
            }
            Event::GeneralRef(r) if in_text_run => {
                if let Some(resolved) = resolve_reference(&r) {
                    text.push_str(&resolved);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(text.trim().to_string())
}

/// Title (`dc:title`) and author (`dc:creator`) from `docProps/core.xml`.
fn core_properties(xml: &str) -> (Option<String>, Option<String>) {
    let mut reader = Reader::from_str(xml);
    let (mut title, mut author) = (String::new(), String::new());
    let mut current: Option<&mut String> = None;

    while let Ok(event) = reader.read_event() {
        match event {
            Event::Start(e) => {
                current = match e.local_name().as_ref() {
                    b"title" => Some(&mut title),
                    b"creator" => Some(&mut author),
                    _ => None,
                }
            }
            Event::End(_) => current = None,
            Event::Text(e) => {
                if let (Some(target), Ok(value)) = (current.as_deref_mut(), e.decode()) {
                    target.push_str(&value);
                }
            }
            Event::GeneralRef(r) => {
                if let (Some(target), Some(value)) = (current.as_deref_mut(), resolve_reference(&r))
                {
                    target.push_str(&value);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    let non_empty = |s: String| Some(s.trim().to_string()).filter(|s| !s.is_empty());
    (non_empty(title), non_empty(author))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCUMENT: &str = r#"<w:document xmlns:w="w"><w:body>
        <w:p><w:r><w:t>Budget &amp; plan</w:t></w:r><w:r><w:tab/><w:t xml:space="preserve"> for Q3</w:t></w:r></w:p>
        <w:p><w:r><w:t>Line one</w:t><w:br/><w:t>Line &#50;</w:t></w:r></w:p>
        <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr></w:p>
    </w:body></w:document>"#;

    #[test]
    fn reads_paragraphs_tabs_breaks_and_entities() {
        assert_eq!(
            body_text(DOCUMENT).unwrap(),
            "Budget & plan\t for Q3\nLine one\nLine 2"
        );
    }

    #[test]
    fn reads_title_and_author() {
        let core = r#"<cp:coreProperties xmlns:dc="dc" xmlns:cp="cp">
            <dc:title>Q3 &amp; Q4 Plan</dc:title><dc:creator>Chidi Eze</dc:creator>
        </cp:coreProperties>"#;
        assert_eq!(
            core_properties(core),
            (Some("Q3 & Q4 Plan".into()), Some("Chidi Eze".into()))
        );
    }

    #[test]
    fn real_test_document_has_metadata() {
        let bytes = std::fs::read(super::super::tests::test_data(
            "documents/Q3 Marketing Plan.docx",
        ))
        .unwrap();
        let extracted = extract(&bytes).unwrap();
        assert_eq!(extracted.title.as_deref(), Some("Q3 Marketing Plan"));
        assert_eq!(extracted.author.as_deref(), Some("Chidi Eze"));
        assert!(extracted.pages[0]
            .text
            .starts_with("Q3 Marketing Plan\nThis quarter"));
    }
}
