//! SQLite storage for Recall's index.
//!
//! `Database` only remembers where the database file is. Each caller opens
//! its own short-lived connection with `connect()`; SQLite's WAL mode lets a
//! background scan write while commands read without blocking each other.
//!
//! The query functions in the submodules take a `&Connection`, so tests can
//! run them against an in-memory database.

pub mod documents;
pub mod embeddings;
pub mod evidence;
pub mod files;
pub mod locations;
pub mod migrations;

/// How much is stored in the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counts {
    pub locations: u64,
    pub files: u64,
    pub documents: u64,
    pub passages: u64,
    pub embeddings: u64,
}

pub fn counts(conn: &Connection) -> rusqlite::Result<Counts> {
    let count = |table: &str| -> rusqlite::Result<u64> {
        let n: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })?;
        Ok(u64::try_from(n).unwrap_or(0))
    };
    Ok(Counts {
        locations: count("locations")?,
        files: count("files")?,
        documents: count("documents")?,
        passages: count("chunks")?,
        embeddings: count("embeddings")?,
    })
}

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

pub use migrations::MigrationError;

/// File name of the database inside the app data folder.
pub const DATABASE_FILE_NAME: &str = "recall.db";

/// How long to wait for another connection's write to finish before failing.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Database {
    path: PathBuf,
}

impl Database {
    /// Opens (creating if needed) the database at `path` and applies migrations.
    pub fn open(path: &Path) -> Result<Self, MigrationError> {
        let database = Self {
            path: path.to_path_buf(),
        };
        let mut conn = database.connect()?;
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        migrations::run(&mut conn)?;
        Ok(database)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Bytes on disk, including SQLite's write-ahead log files.
    pub fn size_on_disk(&self) -> u64 {
        ["", "-wal", "-shm"]
            .iter()
            .filter_map(|suffix| {
                let mut name = self.path.as_os_str().to_owned();
                name.push(suffix);
                std::fs::metadata(PathBuf::from(name)).ok()
            })
            .map(|metadata| metadata.len())
            .sum()
    }

    pub fn connect(&self) -> rusqlite::Result<Connection> {
        let conn = Connection::open(&self.path)?;
        configure(&conn)?;
        Ok(conn)
    }
}

/// Settings every connection needs. Foreign keys are off by default in SQLite.
pub fn configure(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.busy_timeout(BUSY_TIMEOUT)
}

/// A migrated in-memory database for unit tests.
#[cfg(test)]
pub fn test_connection() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    configure(&conn).unwrap();
    migrations::run(&mut conn).unwrap();
    conn
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{add_document, add_location};

    #[test]
    fn counts_what_is_stored() {
        let conn = test_connection();
        let loc = add_location(&conn);
        add_document(&conn, loc, "/a/one.txt", &[(None, "first")]);
        add_document(&conn, loc, "/a/two.pdf", &[(Some(1), "a"), (Some(2), "b")]);
        assert_eq!(
            counts(&conn).unwrap(),
            Counts {
                locations: 1,
                files: 2,
                documents: 2,
                passages: 3,
                embeddings: 0
            }
        );
    }

    #[test]
    fn measures_its_size_on_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = Database::open(&dir.path().join("recall.db")).unwrap();
        assert!(db.size_on_disk() > 0);
    }
}
