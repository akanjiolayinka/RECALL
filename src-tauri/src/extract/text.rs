//! Plain text and Markdown.

use super::{Extracted, Page};

/// Decode text bytes. Handles UTF-8 (with or without a byte-order mark) and
/// UTF-16 with a byte-order mark, which older Windows Notepad saves as
/// "Unicode". Anything else is read as UTF-8, replacing invalid bytes.
pub fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(rest, u16::from_le_bytes);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(rest, u16::from_be_bytes);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn decode_utf16(bytes: &[u8], to_unit: fn([u8; 2]) -> u16) -> String {
    let units = bytes
        .chunks_exact(2)
        .map(|pair| to_unit([pair[0], pair[1]]));
    char::decode_utf16(units)
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Normalise Windows (`\r\n`) and old Mac (`\r`) line endings to `\n`.
fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn extract_plain(bytes: &[u8]) -> Extracted {
    Extracted {
        pages: vec![Page {
            number: None,
            text: normalize_newlines(&decode(bytes)),
        }],
        ..Default::default()
    }
}

/// Markdown is indexed as written; the first `# Heading` becomes the title.
pub fn extract_markdown(bytes: &[u8]) -> Extracted {
    let text = normalize_newlines(&decode(bytes));
    let title = text
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|title| title.trim().to_string())
        .filter(|title| !title.is_empty());
    Extracted {
        pages: vec![Page { number: None, text }],
        title,
        author: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_common_encodings() {
        assert_eq!(decode(b"plain"), "plain");
        assert_eq!(decode(b"\xEF\xBB\xBFwith bom"), "with bom");
        assert_eq!(decode(&[0xFF, 0xFE, b'h', 0, b'i', 0]), "hi");
        assert_eq!(decode(&[0xFE, 0xFF, 0, b'h', 0, b'i']), "hi");
        assert_eq!(decode(b"bad \xFF byte"), "bad \u{FFFD} byte");
    }

    #[test]
    fn normalises_line_endings() {
        assert_eq!(extract_plain(b"a\r\nb\rc").pages[0].text, "a\nb\nc");
    }

    #[test]
    fn markdown_title_is_the_first_heading() {
        let extracted = extract_markdown(b"intro\n## Not this\n# Moving out\ntext");
        assert_eq!(extracted.title.as_deref(), Some("Moving out"));
    }
}
