//! Storage for extracted documents and their chunks.

use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension};

use crate::extract::Extracted;
use crate::files::{FileKind, FileStatus};
use crate::indexing::chunk::Chunk;

/// A file whose contents still need to be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingFile {
    pub id: i64,
    pub path: PathBuf,
    pub kind: FileKind,
}

/// Files in a location waiting to be read. Images are left for OCR.
pub fn pending_files(conn: &Connection, location_id: i64) -> rusqlite::Result<Vec<PendingFile>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, kind FROM files
         WHERE location_id = ?1 AND status = ?2 AND kind != ?3
         ORDER BY modified_at DESC NULLS LAST",
    )?;
    let rows = stmt.query_map(
        params![
            location_id,
            FileStatus::Pending.as_str(),
            FileKind::Image.as_str()
        ],
        |row| {
            let path: String = row.get(1)?;
            let kind: String = row.get(2)?;
            Ok(PendingFile {
                id: row.get(0)?,
                path: PathBuf::from(path),
                kind: FileKind::parse(&kind).unwrap_or(FileKind::Text),
            })
        },
    )?;
    rows.collect()
}

/// Replace a file's document and chunks and mark it indexed. Call inside a
/// transaction so a half-written document is never visible.
pub fn save_document(
    conn: &Connection,
    file_id: i64,
    extracted: &Extracted,
    full_text: &str,
    chunks: &[Chunk],
    indexed_at: i64,
) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM documents WHERE file_id = ?1", [file_id])?;
    conn.execute(
        "INSERT INTO documents (file_id, title, author, page_count, word_count, extracted_text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            file_id,
            extracted.title,
            extracted.author,
            extracted.page_count().map(|n| n as i64),
            extracted.word_count() as i64,
            full_text,
        ],
    )?;
    let document_id = conn.last_insert_rowid();

    let mut insert = conn.prepare(
        "INSERT INTO chunks (document_id, chunk_index, text, page_number, char_start, char_end)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for chunk in chunks {
        insert.execute(params![
            document_id,
            chunk.index as i64,
            chunk.text,
            chunk.page_number,
            chunk.char_start as i64,
            chunk.char_end as i64,
        ])?;
    }

    conn.execute(
        "UPDATE files SET status = ?2, indexed_at = ?3, error = NULL WHERE id = ?1",
        params![file_id, FileStatus::Indexed.as_str(), indexed_at],
    )?;
    Ok(())
}

/// Record that a file's contents couldn't be read, removing any old document.
pub fn mark_unreadable(conn: &Connection, file_id: i64, message: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM documents WHERE file_id = ?1", [file_id])?;
    conn.execute(
        "UPDATE files SET status = ?2, error = ?3, indexed_at = NULL WHERE id = ?1",
        params![file_id, FileStatus::Error.as_str(), message],
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPage {
    pub number: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDocument {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: Option<u32>,
    pub word_count: u32,
    pub chunk_count: u32,
    /// Text of each page that has any text, rebuilt from the chunk positions.
    pub pages: Vec<DocumentPage>,
}

/// The stored text of a file, or `None` if it hasn't been read.
pub fn get_document(conn: &Connection, file_id: i64) -> rusqlite::Result<Option<StoredDocument>> {
    let Some((document_id, title, author, page_count, word_count, full_text)) = conn
        .query_row(
            "SELECT id, title, author, page_count, word_count, extracted_text
             FROM documents WHERE file_id = ?1",
            [file_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<u32>>(3)?,
                    row.get::<_, Option<u32>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(None);
    };

    // Chunks never cross pages, so a page spans from its first chunk's start
    // to its last chunk's end.
    let mut stmt = conn.prepare(
        "SELECT page_number, MIN(char_start), MAX(char_end), COUNT(*) FROM chunks
         WHERE document_id = ?1 GROUP BY page_number ORDER BY MIN(char_start)",
    )?;
    let spans = stmt
        .query_map([document_id], |row| {
            Ok((
                row.get::<_, Option<u32>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, u32>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let chars: Vec<char> = full_text.chars().collect();
    let slice = |start: i64, end: i64| -> String {
        let start = (start.max(0) as usize).min(chars.len());
        let end = (end.max(0) as usize).clamp(start, chars.len());
        chars[start..end].iter().collect()
    };
    let pages = spans
        .iter()
        .map(|&(number, start, end, _)| DocumentPage {
            number,
            text: slice(start, end),
        })
        .collect();

    Ok(Some(StoredDocument {
        title,
        author,
        page_count,
        word_count: word_count.unwrap_or(0),
        chunk_count: spans.iter().map(|span| span.3).sum(),
        pages,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::files::{upsert, ScannedFile};
    use crate::database::{locations, test_connection};
    use crate::extract::Page;
    use crate::indexing::chunk::chunk_pages;
    use std::path::Path;

    fn add_file(conn: &Connection, location_id: i64, path: &str, kind: FileKind) -> i64 {
        upsert(
            conn,
            &ScannedFile {
                location_id,
                path: Path::new(path),
                kind,
                size_bytes: 1,
                created_at: None,
                modified_at: Some(1),
                scanned_at: 1,
                content_hash: Some("h"),
                error: None,
            },
        )
        .unwrap();
        conn.query_row("SELECT id FROM files WHERE path = ?1", [path], |r| r.get(0))
            .unwrap()
    }

    fn extracted() -> Extracted {
        Extracted {
            pages: vec![
                Page {
                    number: Some(1),
                    text: "Garden proposal".into(),
                },
                Page {
                    number: Some(2),
                    text: "Budget is NGN 2,500,000.".into(),
                },
            ],
            title: Some("Proposal".into()),
            author: None,
        }
    }

    #[test]
    fn pending_files_skip_images_and_finished_files() {
        let conn = test_connection();
        let loc = locations::insert(&conn, Path::new("/a"), 0).unwrap().id;
        let pdf = add_file(&conn, loc, "/a/p.pdf", FileKind::Pdf);
        add_file(&conn, loc, "/a/i.png", FileKind::Image);
        let md = add_file(&conn, loc, "/a/n.md", FileKind::Markdown);
        mark_unreadable(&conn, md, "broken").unwrap();

        let pending = pending_files(&conn, loc).unwrap();
        assert_eq!(pending.iter().map(|p| p.id).collect::<Vec<_>>(), vec![pdf]);
    }

    #[test]
    fn saves_and_reads_back_a_document_by_page() {
        let conn = test_connection();
        let loc = locations::insert(&conn, Path::new("/a"), 0).unwrap().id;
        let file = add_file(&conn, loc, "/a/p.pdf", FileKind::Pdf);
        let doc = extracted();
        let (full, chunks) = chunk_pages(&doc.pages);
        save_document(&conn, file, &doc, &full, &chunks, 42).unwrap();

        let stored = get_document(&conn, file).unwrap().unwrap();
        assert_eq!(stored.title.as_deref(), Some("Proposal"));
        assert_eq!(stored.page_count, Some(2));
        assert_eq!(stored.word_count, 6);
        assert_eq!(stored.chunk_count, 2);
        assert_eq!(
            stored.pages,
            vec![
                DocumentPage {
                    number: Some(1),
                    text: "Garden proposal".into()
                },
                DocumentPage {
                    number: Some(2),
                    text: "Budget is NGN 2,500,000.".into()
                },
            ]
        );
        let status: String = conn
            .query_row("SELECT status FROM files WHERE id = ?1", [file], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "indexed");
        assert!(pending_files(&conn, loc).unwrap().is_empty());
    }

    #[test]
    fn saving_again_replaces_and_deleting_the_file_cascades() {
        let conn = test_connection();
        let loc = locations::insert(&conn, Path::new("/a"), 0).unwrap().id;
        let file = add_file(&conn, loc, "/a/p.pdf", FileKind::Pdf);
        let doc = extracted();
        let (full, chunks) = chunk_pages(&doc.pages);
        save_document(&conn, file, &doc, &full, &chunks, 1).unwrap();
        save_document(&conn, file, &doc, &full, &chunks, 2).unwrap();
        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!((count("documents"), count("chunks")), (1, 2));

        locations::delete(&conn, loc).unwrap();
        assert_eq!((count("documents"), count("chunks")), (0, 0));
    }

    #[test]
    fn unreadable_files_have_no_document() {
        let conn = test_connection();
        let loc = locations::insert(&conn, Path::new("/a"), 0).unwrap().id;
        let file = add_file(&conn, loc, "/a/p.pdf", FileKind::Pdf);
        mark_unreadable(&conn, file, "No text found").unwrap();
        assert_eq!(get_document(&conn, file).unwrap(), None);
    }
}
