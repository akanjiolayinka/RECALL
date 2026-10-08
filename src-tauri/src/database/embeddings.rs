//! Storage and nearest-neighbour lookup for passage embeddings.
//!
//! Lookup is a straightforward scan comparing the query with every stored
//! vector. For a personal library (tens of thousands of passages) that takes
//! milliseconds; a vector index can replace it later behind `nearest()`.

use rusqlite::{params, Connection};

use crate::embeddings::{from_bytes, similarity, to_bytes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkToEmbed {
    pub chunk_id: i64,
    pub text: String,
}

// Chunks in a location with no embedding from `model` (a missing row, or one
// made by a different model).
const MISSING: &str = "FROM chunks
     JOIN documents ON documents.id = chunks.document_id
     JOIN files ON files.id = documents.file_id
     LEFT JOIN embeddings ON embeddings.chunk_id = chunks.id AND embeddings.model = ?2
     WHERE files.location_id = ?1 AND embeddings.chunk_id IS NULL";

pub fn count_missing(conn: &Connection, location_id: i64, model: &str) -> rusqlite::Result<usize> {
    let count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) {MISSING}"),
        params![location_id, model],
        |r| r.get(0),
    )?;
    Ok(usize::try_from(count).unwrap_or(0))
}

pub fn next_missing(
    conn: &Connection,
    location_id: i64,
    model: &str,
    limit: usize,
) -> rusqlite::Result<Vec<ChunkToEmbed>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT chunks.id, chunks.text {MISSING} ORDER BY chunks.id LIMIT ?3"
    ))?;
    let rows = stmt.query_map(params![location_id, model, limit as i64], |row| {
        Ok(ChunkToEmbed {
            chunk_id: row.get(0)?,
            text: row.get(1)?,
        })
    })?;
    rows.collect()
}

/// Store (or replace) a chunk's embedding.
pub fn save(conn: &Connection, chunk_id: i64, model: &str, vector: &[f32]) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO embeddings (chunk_id, model, dimensions, embedding) VALUES (?1, ?2, ?3, ?4)",
        params![chunk_id, model, vector.len() as i64, to_bytes(vector)],
    )?;
    Ok(())
}

/// The `limit` chunks most similar to `query` (best first), with their
/// cosine similarity. Only vectors from `model` are compared.
pub fn nearest(
    conn: &Connection,
    model: &str,
    query: &[f32],
    limit: usize,
) -> rusqlite::Result<Vec<(i64, f32)>> {
    let mut stmt = conn.prepare(
        "SELECT chunk_id, embedding FROM embeddings WHERE model = ?1 AND dimensions = ?2",
    )?;
    let mut rows = stmt.query(params![model, query.len() as i64])?;
    let mut scored = Vec::new();
    while let Some(row) = rows.next()? {
        let bytes: Vec<u8> = row.get(1)?;
        scored.push((
            row.get::<_, i64>(0)?,
            similarity(query, &from_bytes(&bytes)),
        ));
    }
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(limit);
    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_connection;
    use crate::test_support::{add_document, add_location};

    #[test]
    fn finds_chunks_missing_an_embedding_for_the_current_model() {
        let conn = test_connection();
        let loc = add_location(&conn);
        add_document(&conn, loc, "/a/one.txt", &[(None, "first")]);
        add_document(&conn, loc, "/a/two.txt", &[(None, "second")]);
        assert_eq!(count_missing(&conn, loc, "model-a").unwrap(), 2);

        let first = next_missing(&conn, loc, "model-a", 1).unwrap();
        assert_eq!(first.len(), 1);
        save(&conn, first[0].chunk_id, "model-a", &[1.0, 0.0]).unwrap();
        assert_eq!(count_missing(&conn, loc, "model-a").unwrap(), 1);
        // A different model needs its own embeddings.
        assert_eq!(count_missing(&conn, loc, "model-b").unwrap(), 2);
    }

    #[test]
    fn nearest_ranks_by_similarity_and_ignores_other_models() {
        let conn = test_connection();
        let loc = add_location(&conn);
        add_document(&conn, loc, "/a/x.txt", &[(None, "x")]);
        add_document(&conn, loc, "/a/y.txt", &[(None, "y")]);
        add_document(&conn, loc, "/a/z.txt", &[(None, "z")]);
        let ids: Vec<i64> = next_missing(&conn, loc, "m", 10)
            .unwrap()
            .iter()
            .map(|c| c.chunk_id)
            .collect();
        save(&conn, ids[0], "m", &[1.0, 0.0]).unwrap();
        save(&conn, ids[1], "m", &[0.6, 0.8]).unwrap();
        save(&conn, ids[2], "other", &[1.0, 0.0]).unwrap();

        let hits = nearest(&conn, "m", &[1.0, 0.0], 10).unwrap();
        assert_eq!(
            hits.iter().map(|h| h.0).collect::<Vec<_>>(),
            vec![ids[0], ids[1]]
        );
        assert!((hits[1].1 - 0.6).abs() < 1e-6);
        assert_eq!(nearest(&conn, "m", &[1.0, 0.0], 1).unwrap().len(), 1);
    }

    #[test]
    fn deleting_a_file_removes_its_embeddings() {
        let conn = test_connection();
        let loc = add_location(&conn);
        let file = add_document(&conn, loc, "/a/x.txt", &[(None, "x")]);
        let chunk = next_missing(&conn, loc, "m", 1).unwrap()[0].chunk_id;
        save(&conn, chunk, "m", &[1.0]).unwrap();
        crate::database::files::delete(&conn, file).unwrap();
        assert!(nearest(&conn, "m", &[1.0], 10).unwrap().is_empty());
    }
}
