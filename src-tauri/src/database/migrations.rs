//! Database schema migrations.
//!
//! Each migration is a SQL file applied exactly once, in order. SQLite's
//! built-in `user_version` number records how many have been applied.
//!
//! To change the schema: add a new numbered file and append it to
//! `MIGRATIONS`. Never edit a migration that has already been released.

use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_initial.sql"),
    include_str!("migrations/0002_keyword_search.sql"),
];

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("database schema version {found} is newer than this version of Recall supports ({supported})")]
    TooNew { found: usize, supported: usize },
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

/// The schema version this build of Recall expects.
pub const LATEST_VERSION: usize = MIGRATIONS.len();

pub fn current_version(conn: &Connection) -> rusqlite::Result<usize> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(usize::try_from(version).unwrap_or(0))
}

/// Bring the database up to `LATEST_VERSION`. Each migration runs in its own
/// transaction, so a failure leaves the database at the previous version.
pub fn run(conn: &mut Connection) -> Result<(), MigrationError> {
    let current = current_version(conn)?;
    if current > LATEST_VERSION {
        return Err(MigrationError::TooNew {
            found: current,
            supported: LATEST_VERSION,
        });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_names(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            // FTS5 also creates internal "shadow" tables (chunks_fts_data, ...).
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table'
                 AND name NOT LIKE 'chunks_fts_%' ORDER BY name",
            )
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn creates_all_tables_on_a_fresh_database() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), LATEST_VERSION);
        assert_eq!(
            table_names(&conn),
            vec![
                "chunks",
                "chunks_fts",
                "documents",
                "embeddings",
                "files",
                "locations"
            ]
        );
    }

    #[test]
    fn upgrading_indexes_chunks_that_already_exist() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute_batch(
            "INSERT INTO locations VALUES (1, '/a', 0);
             INSERT INTO files (id, location_id, path, filename, extension, kind, mime_type,
                                size_bytes, scanned_at, status)
                 VALUES (1, 1, '/a/n.txt', 'n.txt', 'txt', 'text', 'text/plain', 1, 0, 'indexed');
             INSERT INTO documents (id, file_id, extracted_text) VALUES (1, 1, 'garden budget');
             INSERT INTO chunks (document_id, chunk_index, text, char_start, char_end)
                 VALUES (1, 0, 'garden budget', 0, 13);",
        )
        .unwrap();

        run(&mut conn).unwrap();
        let hits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM chunks_fts WHERE chunks_fts MATCH 'budget'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);
    }

    #[test]
    fn running_twice_is_harmless() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();
        run(&mut conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), LATEST_VERSION);
    }

    #[test]
    fn refuses_a_database_from_a_newer_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", (LATEST_VERSION + 1) as i64)
            .unwrap();
        assert!(matches!(run(&mut conn), Err(MigrationError::TooNew { .. })));
    }
}
