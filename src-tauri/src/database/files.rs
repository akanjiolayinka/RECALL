//! Storage for files found by the scanner.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::files::kind::mime_type;
use crate::files::{FileKind, FileRecord, FileStatus};

/// What the database already knows about a file, used to decide whether it
/// needs to be fingerprinted again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownFile {
    pub id: i64,
    pub size_bytes: u64,
    pub modified_at: Option<i64>,
}

impl KnownFile {
    /// True if the file on disk still has the same size and modification
    /// time, so its stored fingerprint can be trusted without re-reading it.
    pub fn looks_unchanged(&self, size_bytes: u64, modified_at: Option<i64>) -> bool {
        modified_at.is_some() && self.size_bytes == size_bytes && self.modified_at == modified_at
    }
}

/// Everything the scanner needs to save about one file.
#[derive(Debug)]
pub struct ScannedFile<'a> {
    pub location_id: i64,
    pub path: &'a Path,
    pub kind: FileKind,
    pub size_bytes: u64,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub scanned_at: i64,
    /// `None` when the file couldn't be read (then `error` explains why).
    pub content_hash: Option<&'a str>,
    pub error: Option<&'a str>,
}

/// Files already stored for a location, keyed by path.
pub fn known_files(
    conn: &Connection,
    location_id: i64,
) -> rusqlite::Result<HashMap<PathBuf, KnownFile>> {
    let mut stmt =
        conn.prepare("SELECT id, path, size_bytes, modified_at FROM files WHERE location_id = ?1")?;
    let rows = stmt.query_map([location_id], |row| {
        let path: String = row.get(1)?;
        let size: i64 = row.get(2)?;
        Ok((
            PathBuf::from(path),
            KnownFile {
                id: row.get(0)?,
                size_bytes: size.max(0) as u64,
                modified_at: row.get(3)?,
            },
        ))
    })?;
    rows.collect()
}

/// Insert a new file, or update a known one.
///
/// If the contents changed (different fingerprint), the file goes back to
/// `pending` so it gets re-indexed. Unreadable files are marked `error`.
pub fn upsert(conn: &Connection, file: &ScannedFile<'_>) -> rusqlite::Result<()> {
    let filename = file
        .path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let extension = file
        .path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let status = if file.error.is_some() {
        FileStatus::Error
    } else {
        FileStatus::Pending
    };

    conn.execute(
        "INSERT INTO files (location_id, path, filename, extension, kind, mime_type, size_bytes,
                            created_at, modified_at, scanned_at, content_hash, status, error)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT (path) DO UPDATE SET
             location_id  = excluded.location_id,
             size_bytes   = excluded.size_bytes,
             created_at   = excluded.created_at,
             modified_at  = excluded.modified_at,
             scanned_at   = excluded.scanned_at,
             error        = excluded.error,
             status       = CASE
                 WHEN excluded.status = 'error' THEN 'error'
                 WHEN files.content_hash IS excluded.content_hash AND files.status != 'error'
                     THEN files.status
                 ELSE 'pending'
             END,
             indexed_at   = CASE
                 WHEN files.content_hash IS excluded.content_hash THEN files.indexed_at
                 ELSE NULL
             END,
             content_hash = excluded.content_hash",
        params![
            file.location_id,
            file.path.to_string_lossy(),
            filename,
            extension,
            file.kind.as_str(),
            mime_type(&extension),
            i64::try_from(file.size_bytes).unwrap_or(i64::MAX),
            file.created_at,
            file.modified_at,
            file.scanned_at,
            file.content_hash,
            status.as_str(),
            file.error,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM files WHERE id = ?1", [id])?;
    Ok(())
}

/// Number of files stored for each location.
pub fn count_by_location(conn: &Connection) -> rusqlite::Result<HashMap<i64, usize>> {
    let mut stmt = conn.prepare("SELECT location_id, COUNT(*) FROM files GROUP BY location_id")?;
    let rows = stmt.query_map([], |row| {
        let count: i64 = row.get(1)?;
        Ok((row.get(0)?, usize::try_from(count).unwrap_or(0)))
    })?;
    rows.collect()
}

/// Filters for listing files. All fields are optional.
#[derive(Debug, Default, Clone)]
pub struct FileQuery {
    pub location_id: Option<i64>,
    pub kind: Option<FileKind>,
    /// Case-insensitive match against the file name (ASCII letters only, a
    /// limitation of SQLite's built-in `lower()`).
    pub name_contains: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

/// Default and maximum page size, so the UI never receives an unbounded list.
pub const DEFAULT_LIMIT: usize = 100;
pub const MAX_LIMIT: usize = 500;

// One WHERE clause shared by the page and count queries. A NULL parameter
// means "don't filter on this".
const FILTER: &str = "(?1 IS NULL OR location_id = ?1)
    AND (?2 IS NULL OR kind = ?2)
    AND (?3 IS NULL OR instr(lower(filename), lower(?3)) > 0)";

/// One page of matching files (most recently modified first) and the total
/// number of matches.
pub fn query(conn: &Connection, query: &FileQuery) -> rusqlite::Result<(Vec<FileRecord>, usize)> {
    let name = query
        .name_contains
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty());
    let kind = query.kind.map(FileKind::as_str);
    let limit = i64::try_from(query.limit.clamp(1, MAX_LIMIT)).unwrap_or(1);
    let offset = i64::try_from(query.offset).unwrap_or(0);

    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM files WHERE {FILTER}"),
        params![query.location_id, kind, name],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(&format!(
        "SELECT id, location_id, path, kind, size_bytes, modified_at, created_at, status, error
         FROM files WHERE {FILTER}
         ORDER BY modified_at DESC NULLS LAST, path
         LIMIT ?4 OFFSET ?5"
    ))?;
    let rows = stmt.query_map(
        params![query.location_id, kind, name, limit, offset],
        |row| {
            let path: String = row.get(2)?;
            let kind: String = row.get(3)?;
            let size: i64 = row.get(4)?;
            let status: String = row.get(7)?;
            Ok(FileRecord {
                id: row.get(0)?,
                location_id: row.get(1)?,
                path: PathBuf::from(path),
                // Only values written by `upsert` are stored, so these always parse.
                kind: FileKind::parse(&kind).unwrap_or(FileKind::Text),
                size_bytes: size.max(0) as u64,
                modified_at: row.get(5)?,
                created_at: row.get(6)?,
                status: FileStatus::parse(&status).unwrap_or(FileStatus::Pending),
                error: row.get(8)?,
            })
        },
    )?;
    let files = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok((files, usize::try_from(total).unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{locations, test_connection};

    fn scanned<'a>(
        location_id: i64,
        path: &'a str,
        kind: FileKind,
        modified_at: i64,
        hash: &'a str,
    ) -> ScannedFile<'a> {
        ScannedFile {
            location_id,
            path: Path::new(path),
            kind,
            size_bytes: 10,
            created_at: None,
            modified_at: Some(modified_at),
            scanned_at: 1,
            content_hash: Some(hash),
            error: None,
        }
    }

    fn status_of(conn: &Connection, path: &str) -> String {
        conn.query_row("SELECT status FROM files WHERE path = ?1", [path], |row| {
            row.get(0)
        })
        .unwrap()
    }

    fn seeded() -> (Connection, i64, i64) {
        let conn = test_connection();
        let a = locations::insert(&conn, Path::new("/a"), 0).unwrap().id;
        let b = locations::insert(&conn, Path::new("/b"), 0).unwrap().id;
        upsert(
            &conn,
            &scanned(a, "/a/Budget 2024.pdf", FileKind::Pdf, 100, "h1"),
        )
        .unwrap();
        upsert(
            &conn,
            &scanned(a, "/a/notes.md", FileKind::Markdown, 300, "h2"),
        )
        .unwrap();
        upsert(
            &conn,
            &scanned(b, "/b/budget.png", FileKind::Image, 200, "h3"),
        )
        .unwrap();
        (conn, a, b)
    }

    fn names(files: &[FileRecord]) -> Vec<String> {
        files.iter().map(FileRecord::file_name).collect()
    }

    #[test]
    fn lists_newest_first_with_metadata() {
        let (conn, _, _) = seeded();
        let (files, total) = query(
            &conn,
            &FileQuery {
                limit: DEFAULT_LIMIT,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(total, 3);
        assert_eq!(
            names(&files),
            vec!["notes.md", "budget.png", "Budget 2024.pdf"]
        );
        assert_eq!(files[0].kind, FileKind::Markdown);
        assert_eq!(files[0].status, FileStatus::Pending);
        let mime: String = conn
            .query_row(
                "SELECT mime_type FROM files WHERE filename = 'notes.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mime, "text/markdown");
    }

    #[test]
    fn filters_by_location_kind_and_name() {
        let (conn, a, _) = seeded();
        let by_name = FileQuery {
            name_contains: Some(" BUDGET ".into()),
            limit: 10,
            ..Default::default()
        };
        assert_eq!(
            names(&query(&conn, &by_name).unwrap().0),
            vec!["budget.png", "Budget 2024.pdf"]
        );

        let by_kind_and_location = FileQuery {
            location_id: Some(a),
            kind: Some(FileKind::Pdf),
            limit: 10,
            ..Default::default()
        };
        assert_eq!(
            names(&query(&conn, &by_kind_and_location).unwrap().0),
            vec!["Budget 2024.pdf"]
        );
    }

    #[test]
    fn pages_results_but_reports_full_total() {
        let (conn, _, _) = seeded();
        let (files, total) = query(
            &conn,
            &FileQuery {
                limit: 2,
                offset: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(total, 3);
        assert_eq!(names(&files), vec!["budget.png", "Budget 2024.pdf"]);
    }

    #[test]
    fn changed_content_resets_status_to_pending() {
        let (conn, a, _) = seeded();
        conn.execute(
            "UPDATE files SET status = 'indexed', indexed_at = 5 WHERE path = '/a/notes.md'",
            [],
        )
        .unwrap();

        // Same fingerprint: stays indexed.
        upsert(
            &conn,
            &scanned(a, "/a/notes.md", FileKind::Markdown, 301, "h2"),
        )
        .unwrap();
        assert_eq!(status_of(&conn, "/a/notes.md"), "indexed");

        // New fingerprint: needs indexing again.
        upsert(
            &conn,
            &scanned(a, "/a/notes.md", FileKind::Markdown, 302, "h2-new"),
        )
        .unwrap();
        assert_eq!(status_of(&conn, "/a/notes.md"), "pending");
        let indexed_at: Option<i64> = conn
            .query_row(
                "SELECT indexed_at FROM files WHERE path = '/a/notes.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(indexed_at, None);
    }

    #[test]
    fn unreadable_files_are_marked_error_and_recover() {
        let (conn, a, _) = seeded();
        let mut broken = scanned(a, "/a/notes.md", FileKind::Markdown, 300, "");
        broken.content_hash = None;
        broken.error = Some("Recall isn't allowed to read this file.");
        upsert(&conn, &broken).unwrap();
        assert_eq!(status_of(&conn, "/a/notes.md"), "error");

        upsert(
            &conn,
            &scanned(a, "/a/notes.md", FileKind::Markdown, 300, "h2"),
        )
        .unwrap();
        assert_eq!(status_of(&conn, "/a/notes.md"), "pending");
    }

    #[test]
    fn known_files_counts_and_cascading_delete() {
        let (conn, a, b) = seeded();
        let known = known_files(&conn, a).unwrap();
        assert_eq!(known.len(), 2);
        let notes = &known[Path::new("/a/notes.md")];
        assert!(notes.looks_unchanged(10, Some(300)));
        assert!(!notes.looks_unchanged(11, Some(300)));
        assert!(!notes.looks_unchanged(10, Some(301)));

        assert_eq!(
            count_by_location(&conn).unwrap(),
            HashMap::from([(a, 2), (b, 1)])
        );
        delete(&conn, notes.id).unwrap();
        locations::delete(&conn, b).unwrap();
        assert_eq!(count_by_location(&conn).unwrap(), HashMap::from([(a, 1)]));
    }

    #[test]
    fn unknown_modification_time_is_never_trusted() {
        let known = KnownFile {
            id: 1,
            size_bytes: 10,
            modified_at: None,
        };
        assert!(!known.looks_unchanged(10, None));
    }
}
