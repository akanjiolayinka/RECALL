//! Storage for locations (folders being indexed).

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::locations::Location;

fn row_to_location(row: &rusqlite::Row<'_>) -> rusqlite::Result<Location> {
    let path: String = row.get(1)?;
    Ok(Location {
        id: row.get(0)?,
        path: PathBuf::from(path),
    })
}

/// All locations, oldest first.
pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Location>> {
    let mut stmt = conn.prepare("SELECT id, path FROM locations ORDER BY id")?;
    let rows = stmt.query_map([], row_to_location)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Location>> {
    conn.query_row(
        "SELECT id, path FROM locations WHERE id = ?1",
        [id],
        row_to_location,
    )
    .optional()
}

pub fn insert(conn: &Connection, path: &Path, added_at: i64) -> rusqlite::Result<Location> {
    conn.execute(
        "INSERT INTO locations (path, added_at) VALUES (?1, ?2)",
        params![path.to_string_lossy(), added_at],
    )?;
    Ok(Location {
        id: conn.last_insert_rowid(),
        path: path.to_path_buf(),
    })
}

/// Deletes a location and, through `ON DELETE CASCADE`, everything indexed
/// from it. Returns whether a location was deleted.
pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    Ok(conn.execute("DELETE FROM locations WHERE id = ?1", [id])? > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_connection;

    #[test]
    fn inserts_lists_gets_and_deletes() {
        let conn = test_connection();
        let docs = insert(&conn, Path::new("/home/me/docs"), 1).unwrap();
        let pics = insert(&conn, Path::new("/home/me/pics"), 2).unwrap();
        assert_eq!(list(&conn).unwrap(), vec![docs.clone(), pics.clone()]);
        assert_eq!(get(&conn, docs.id).unwrap(), Some(docs.clone()));

        assert!(delete(&conn, docs.id).unwrap());
        assert!(!delete(&conn, docs.id).unwrap());
        assert_eq!(list(&conn).unwrap(), vec![pics]);
        assert_eq!(get(&conn, docs.id).unwrap(), None);
    }

    #[test]
    fn rejects_the_same_path_twice() {
        let conn = test_connection();
        insert(&conn, Path::new("/home/me/docs"), 1).unwrap();
        assert!(insert(&conn, Path::new("/home/me/docs"), 2).is_err());
    }
}
