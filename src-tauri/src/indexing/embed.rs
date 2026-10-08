//! Embedding the passages of a location, in batches.

use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;

use crate::database::embeddings;
use crate::embeddings::{EmbedError, Embedder};

/// Passages embedded per model call.
const BATCH_SIZE: usize = 16;

#[derive(Debug)]
pub enum EmbedStop {
    Cancelled,
    Model(EmbedError),
    Database(rusqlite::Error),
}

impl From<rusqlite::Error> for EmbedStop {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err)
    }
}

/// Embed every passage in `location_id` that has no embedding from this
/// model yet. `progress(done, total)` is called after each batch.
pub fn embed_location(
    conn: &mut Connection,
    embedder: &dyn Embedder,
    location_id: i64,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> Result<usize, EmbedStop> {
    let model = embedder.model_id();
    let total = embeddings::count_missing(conn, location_id, model)?;
    let mut done = 0;
    progress(done, total);

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(EmbedStop::Cancelled);
        }
        let batch = embeddings::next_missing(conn, location_id, model, BATCH_SIZE)?;
        if batch.is_empty() {
            break;
        }
        let texts: Vec<&str> = batch.iter().map(|c| c.text.as_str()).collect();
        let vectors = embedder.embed_passages(&texts).map_err(EmbedStop::Model)?;
        if vectors.len() != batch.len() || vectors.iter().any(|v| v.len() != embedder.dimensions())
        {
            return Err(EmbedStop::Model(EmbedError(format!(
                "model returned {} vectors for {} passages, or vectors of the wrong size",
                vectors.len(),
                batch.len()
            ))));
        }
        let tx = conn.transaction()?;
        for (chunk, vector) in batch.iter().zip(&vectors) {
            embeddings::save(&tx, chunk.chunk_id, model, vector)?;
        }
        tx.commit()?;
        done += batch.len();
        progress(done, total);
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_connection;
    use crate::embeddings::fake::FakeEmbedder;
    use crate::test_support::{add_document, add_location};

    #[test]
    fn embeds_every_passage_once_and_again_after_a_model_change() {
        let mut conn = test_connection();
        let loc = add_location(&conn);
        for i in 0..20 {
            add_document(
                &conn,
                loc,
                &format!("/a/{i}.txt"),
                &[(None, "garden budget notes")],
            );
        }
        let cancel = AtomicBool::new(false);
        let mut calls = Vec::new();

        let v1 = FakeEmbedder::new("fake-v1");
        assert_eq!(
            embed_location(&mut conn, &v1, loc, &cancel, |d, t| calls.push((d, t))).unwrap(),
            20
        );
        assert_eq!(calls, vec![(0, 20), (16, 20), (20, 20)]);
        // Nothing left to do for the same model.
        assert_eq!(
            embed_location(&mut conn, &v1, loc, &cancel, |_, _| {}).unwrap(),
            0
        );
        // A new model re-embeds everything.
        let v2 = FakeEmbedder::new("fake-v2");
        assert_eq!(
            embed_location(&mut conn, &v2, loc, &cancel, |_, _| {}).unwrap(),
            20
        );
    }

    #[test]
    fn stops_when_cancelled() {
        let mut conn = test_connection();
        let loc = add_location(&conn);
        add_document(&conn, loc, "/a/x.txt", &[(None, "text")]);
        let result = embed_location(
            &mut conn,
            &FakeEmbedder::new("f"),
            loc,
            &AtomicBool::new(true),
            |_, _| {},
        );
        assert!(matches!(result, Err(EmbedStop::Cancelled)));
    }
}
