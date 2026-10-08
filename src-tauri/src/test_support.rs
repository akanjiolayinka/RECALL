//! Helpers shared by unit tests: build a small in-memory library.

use std::path::Path;

use rusqlite::Connection;

use crate::database::documents::save_document;
use crate::database::files::{upsert, ScannedFile};
use crate::database::locations;
use crate::extract::{Extracted, Page};
use crate::files::FileKind;
use crate::indexing::chunk::chunk_pages;

pub fn add_location(conn: &Connection) -> i64 {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM locations", [], |r| r.get(0))
        .unwrap();
    locations::insert(conn, Path::new(&format!("/library-{count}")), 0)
        .unwrap()
        .id
}

/// Add a file without reading its text (e.g. an image). Returns its id.
pub fn add_file(conn: &Connection, location: i64, path: &str) -> i64 {
    let kind = FileKind::from_path(Path::new(path)).expect("supported test file extension");
    upsert(
        conn,
        &ScannedFile {
            location_id: location,
            path: Path::new(path),
            kind,
            size_bytes: 1,
            created_at: None,
            modified_at: Some(1),
            scanned_at: 1,
            content_hash: Some(path),
            error: None,
        },
    )
    .unwrap();
    conn.query_row("SELECT id FROM files WHERE path = ?1", [path], |r| r.get(0))
        .unwrap()
}

/// Add a file whose text is `pages` (page number, text). Returns its id.
pub fn add_document(
    conn: &Connection,
    location: i64,
    path: &str,
    pages: &[(Option<u32>, &str)],
) -> i64 {
    let id = add_file(conn, location, path);
    let extracted = Extracted {
        pages: pages
            .iter()
            .map(|&(number, text)| Page {
                number,
                text: text.into(),
            })
            .collect(),
        ..Default::default()
    };
    let (full, chunks) = chunk_pages(&extracted.pages);
    save_document(conn, id, &extracted, &full, &chunks, 1).unwrap();
    id
}
