//! Meaning-based search: passages whose embeddings are closest to the query's.

use std::collections::HashSet;
use std::path::PathBuf;

use rusqlite::Connection;

use crate::database::embeddings;
use crate::embeddings::Embedder;
use crate::files::FileKind;

/// Passages less similar than this are not treated as meaning matches.
///
/// TODO(Milestone 7 checkpoint): PLACEHOLDER, not calibrated. Similarity
/// scales differ between models; pick this by testing BGE-small-en-v1.5 on
/// test-data/ (related vs unrelated queries) once the real model is wired in.
pub const MIN_SIMILARITY: f32 = 0.5;

/// Nearest passages considered before keeping one per file.
const CANDIDATES: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticHit {
    pub chunk_id: i64,
    pub file_id: i64,
    pub path: PathBuf,
    pub kind: FileKind,
    pub page_number: Option<u32>,
    pub text: String,
    /// Cosine similarity, -1..1 (higher is closer in meaning).
    pub similarity: f32,
}

/// The most similar passage of each file, best first. Model failures are
/// logged and give no results, so keyword search still works.
pub fn search(
    conn: &Connection,
    embedder: &dyn Embedder,
    query: &str,
) -> rusqlite::Result<Vec<SemanticHit>> {
    let query_vector = match embedder.embed_query(query) {
        Ok(vector) => vector,
        Err(err) => {
            eprintln!("recall: could not embed search query: {err}");
            return Ok(Vec::new());
        }
    };
    let nearest = embeddings::nearest(conn, embedder.model_id(), &query_vector, CANDIDATES)?;

    let mut seen_files = HashSet::new();
    let mut hits = Vec::new();
    for (chunk_id, similarity) in nearest {
        if similarity < MIN_SIMILARITY {
            break; // sorted best first, so the rest are lower
        }
        let hit = passage(conn, chunk_id, similarity)?;
        if seen_files.insert(hit.file_id) {
            hits.push(hit);
        }
    }
    Ok(hits)
}

fn passage(conn: &Connection, chunk_id: i64, similarity: f32) -> rusqlite::Result<SemanticHit> {
    conn.query_row(
        "SELECT files.id, files.path, files.kind, chunks.page_number, chunks.text
         FROM chunks
         JOIN documents ON documents.id = chunks.document_id
         JOIN files ON files.id = documents.file_id
         WHERE chunks.id = ?1",
        [chunk_id],
        |row| {
            Ok(SemanticHit {
                chunk_id,
                file_id: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                kind: FileKind::parse(&row.get::<_, String>(2)?).unwrap_or(FileKind::Text),
                page_number: row.get(3)?,
                text: row.get(4)?,
                similarity,
            })
        },
    )
}
