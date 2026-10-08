//! Evidence: the exact stored text that supports a search result.

use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension};

use crate::files::FileKind;

/// A passage shown inside the text of its page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub file_id: i64,
    pub path: PathBuf,
    pub kind: FileKind,
    pub page_number: Option<u32>,
    /// Text of the page before the passage.
    pub before: String,
    /// The passage itself.
    pub passage: String,
    /// Text of the page after the passage.
    pub after: String,
}

/// The evidence for passage `chunk_id`, or `None` if it no longer exists or
/// doesn't line up with its document (never show text we can't stand behind).
pub fn get(conn: &Connection, chunk_id: i64) -> rusqlite::Result<Option<Evidence>> {
    let Some((document_id, page, start, end, chunk_text, file_id, path, kind, full_text)) = conn
        .query_row(
            "SELECT chunks.document_id, chunks.page_number, chunks.char_start, chunks.char_end, chunks.text,
                    files.id, files.path, files.kind, documents.extracted_text
             FROM chunks
             JOIN documents ON documents.id = chunks.document_id
             JOIN files ON files.id = documents.file_id
             WHERE chunks.id = ?1",
            [chunk_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<u32>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(None);
    };

    // Chunks never cross pages, so the page spans its chunks' offsets.
    let (page_start, page_end): (i64, i64) = conn.query_row(
        "SELECT MIN(char_start), MAX(char_end) FROM chunks
         WHERE document_id = ?1 AND page_number IS ?2",
        rusqlite::params![document_id, page],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let chars: Vec<char> = full_text.chars().collect();
    let [page_start, start, end, page_end] =
        [page_start, start, end, page_end].map(|i| i.max(0) as usize);
    if !(page_start <= start && start <= end && end <= page_end && page_end <= chars.len()) {
        return Ok(None);
    }
    let text = |from: usize, to: usize| chars[from..to].iter().collect::<String>();
    let passage = text(start, end);
    if passage != chunk_text {
        return Ok(None);
    }
    Ok(Some(Evidence {
        file_id,
        path: PathBuf::from(path),
        kind: FileKind::parse(&kind).unwrap_or(FileKind::Text),
        page_number: page,
        before: text(page_start, start),
        passage,
        after: text(end, page_end),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_connection;
    use crate::test_support::{add_document, add_location};

    fn chunk_ids(conn: &Connection, file_id: i64) -> Vec<i64> {
        let mut stmt = conn
            .prepare(
                "SELECT chunks.id FROM chunks JOIN documents ON documents.id = chunks.document_id
                 WHERE documents.file_id = ?1 ORDER BY chunk_index",
            )
            .unwrap();
        stmt.query_map([file_id], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn shows_the_passage_within_its_page() {
        let conn = test_connection();
        let loc = add_location(&conn);
        let file = add_document(
            &conn,
            loc,
            "/d/p.pdf",
            &[
                (Some(1), "Intro page."),
                (Some(2), "Budget is NGN 2,500,000 🌱."),
            ],
        );
        let ids = chunk_ids(&conn, file);
        let evidence = get(&conn, ids[1]).unwrap().unwrap();
        assert_eq!(evidence.page_number, Some(2));
        assert_eq!(evidence.passage, "Budget is NGN 2,500,000 🌱.");
        assert_eq!(
            (evidence.before.as_str(), evidence.after.as_str()),
            ("", "")
        );
        assert_eq!(evidence.file_id, file);
    }

    #[test]
    fn long_pages_give_context_before_and_after() {
        let conn = test_connection();
        let loc = add_location(&conn);
        let text = (0..120)
            .map(|i| format!("Sentence {i} about the garden."))
            .collect::<Vec<_>>()
            .join(" ");
        let file = add_document(&conn, loc, "/d/n.txt", &[(None, &text)]);
        let ids = chunk_ids(&conn, file);
        assert!(ids.len() >= 3);
        let middle = get(&conn, ids[1]).unwrap().unwrap();
        assert!(!middle.before.is_empty() && !middle.after.is_empty());
        // Overlap means before + passage + after may repeat a little text, but
        // the page always starts and ends where the document does.
        assert!(middle.before.starts_with("Sentence 0"));
        assert!(middle.after.ends_with("Sentence 119 about the garden."));
    }

    #[test]
    fn missing_or_inconsistent_passages_give_no_evidence() {
        let conn = test_connection();
        let loc = add_location(&conn);
        let file = add_document(&conn, loc, "/d/n.txt", &[(None, "Original text.")]);
        let id = chunk_ids(&conn, file)[0];
        assert!(get(&conn, 999_999).unwrap().is_none());
        conn.execute(
            "UPDATE chunks SET text = 'Something else.' WHERE id = ?1",
            [id],
        )
        .unwrap();
        assert!(get(&conn, id).unwrap().is_none());
    }
}
