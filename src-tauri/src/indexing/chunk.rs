//! Splitting extracted text into chunks: the passages that get searched,
//! embedded and shown as evidence.
//!
//! Chunks are roughly `TARGET_CHARS` long, end at the most natural break
//! available (paragraph, line, sentence, then word), overlap their neighbour
//! slightly so a sentence cut at a boundary is still found, and never span
//! two pages, so every chunk has one exact page number.

use crate::extract::Page;

/// Aim for chunks of about this many characters (~250 English words), well
/// within the 512-token input limit of small embedding models. Measured in
/// UTF-8 bytes, so chunks of non-English text come out a little shorter.
pub const TARGET_CHARS: usize = 1000;
/// How far back from the target we look for a natural break.
const SEARCH_WINDOW: usize = 300;
/// Characters repeated at the start of the next chunk.
const OVERLAP_CHARS: usize = 150;

/// Separator placed between pages in the document's full text.
pub const PAGE_SEPARATOR: &str = "\n\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub index: usize,
    pub text: String,
    pub page_number: Option<u32>,
    /// Character (not byte) offsets into the document's full text.
    pub char_start: usize,
    pub char_end: usize,
}

/// Join pages into the document's full text and split it into chunks.
pub fn chunk_pages(pages: &[Page]) -> (String, Vec<Chunk>) {
    let mut full_text = String::new();
    let mut full_chars = 0;
    let mut chunks = Vec::new();

    for (i, page) in pages.iter().enumerate() {
        if i > 0 {
            full_text.push_str(PAGE_SEPARATOR);
            full_chars += PAGE_SEPARATOR.chars().count();
        }
        for (start, end) in split(&page.text) {
            let text = &page.text[start..end];
            let char_start = full_chars + page.text[..start].chars().count();
            chunks.push(Chunk {
                index: chunks.len(),
                text: text.to_string(),
                page_number: page.number,
                char_start,
                char_end: char_start + text.chars().count(),
            });
        }
        full_text.push_str(&page.text);
        full_chars += page.text.chars().count();
    }
    (full_text, chunks)
}

/// Byte ranges of the chunks of one page, trimmed of surrounding whitespace.
fn split(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = if text.len() - start <= TARGET_CHARS {
            text.len()
        } else {
            break_point(text, start)
        };
        if let Some(range) = trimmed(text, start, end) {
            ranges.push(range);
        }
        if end >= text.len() {
            break;
        }
        // Step back for overlap, then forward to the start of a word.
        let overlap_start =
            floor_char_boundary(text, end.saturating_sub(OVERLAP_CHARS)).max(start + 1);
        let next = text[overlap_start..end]
            .find(char::is_whitespace)
            .map(|i| overlap_start + i)
            .unwrap_or(end);
        start = ceil_char_boundary(text, next.max(start + 1));
    }
    ranges
}

/// Where to end a chunk starting at `start`: the latest natural break between
/// `TARGET - SEARCH_WINDOW` and `TARGET`, or a hard cut if there is none.
fn break_point(text: &str, start: usize) -> usize {
    let target = floor_char_boundary(text, start + TARGET_CHARS);
    let window_start = floor_char_boundary(text, target - SEARCH_WINDOW);
    let window = &text[window_start..target];
    for separator in ["\n\n", "\n", ". ", "? ", "! ", " "] {
        if let Some(i) = window.rfind(separator) {
            return window_start + i + separator.len();
        }
    }
    target
}

fn trimmed(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let slice = &text[start..end];
    let leading = slice.len() - slice.trim_start().len();
    let trailing = slice.len() - slice.trim_end().len();
    (leading + trailing < slice.len()).then_some((start + leading, end - trailing))
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(number: Option<u32>, text: &str) -> Page {
        Page {
            number,
            text: text.to_string(),
        }
    }

    /// Each chunk's offsets must point at exactly its text in the full text.
    fn assert_offsets_match(full: &str, chunks: &[Chunk]) {
        for chunk in chunks {
            let at_offsets: String = full
                .chars()
                .skip(chunk.char_start)
                .take(chunk.char_end - chunk.char_start)
                .collect();
            assert_eq!(at_offsets, chunk.text, "offsets of chunk {}", chunk.index);
        }
    }

    fn long_text(sentences: usize) -> String {
        (0..sentences)
            .map(|i| {
                format!("Sentence number {i} talks about the garden budget and the water tank.")
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn short_text_is_one_chunk() {
        let (full, chunks) = chunk_pages(&[page(None, "  Just a short note.  ")]);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Just a short note.");
        assert_offsets_match(&full, &chunks);
    }

    #[test]
    fn long_text_splits_at_sentences_within_size_with_overlap() {
        let text = long_text(80);
        let (full, chunks) = chunk_pages(&[page(None, &text)]);
        assert!(chunks.len() > 3);
        for pair in chunks.windows(2) {
            assert!(pair[0].text.chars().count() <= TARGET_CHARS);
            assert!(
                pair[0].text.ends_with('.'),
                "breaks after a sentence: {:?}",
                pair[0].text
            );
            assert!(
                pair[1].char_start < pair[0].char_end,
                "neighbouring chunks overlap"
            );
        }
        // Every word of the text is in some chunk.
        assert!(chunks.last().unwrap().text.ends_with("water tank."));
        assert_offsets_match(&full, &chunks);
    }

    #[test]
    fn chunks_never_span_pages_and_keep_page_numbers() {
        let pages = [
            page(Some(1), &long_text(20)),
            page(Some(2), "Budget: NGN 2,500,000."),
        ];
        let (full, chunks) = chunk_pages(&pages);
        let last = chunks.last().unwrap();
        assert_eq!(last.text, "Budget: NGN 2,500,000.");
        assert_eq!(last.page_number, Some(2));
        assert!(chunks[..chunks.len() - 1]
            .iter()
            .all(|c| c.page_number == Some(1)));
        assert_offsets_match(&full, &chunks);
    }

    #[test]
    fn handles_non_ascii_text_without_splitting_characters() {
        let text = "Ẹ kú àárọ̀ — ọgbà ilé wa 🌱. ".repeat(80);
        let (full, chunks) = chunk_pages(&[page(None, &text)]);
        assert!(chunks.len() > 1);
        assert_offsets_match(&full, &chunks);
    }

    #[test]
    fn a_huge_word_is_cut_rather_than_looping_forever() {
        let text = "x".repeat(5000);
        let (full, chunks) = chunk_pages(&[page(None, &text)]);
        assert!(chunks.len() >= 5);
        assert_offsets_match(&full, &chunks);
    }

    #[test]
    fn blank_pages_produce_no_chunks() {
        let (_, chunks) = chunk_pages(&[page(Some(1), "   \n "), page(Some(2), "Text")]);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].page_number, Some(2));
    }
}
