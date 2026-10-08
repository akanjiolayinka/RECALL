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
pub mod files;
pub mod locations;
pub mod migrations;

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
