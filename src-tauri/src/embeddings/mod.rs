//! Embeddings: turning text into vectors whose closeness reflects meaning.
//!
//! Everything else in Recall talks to embeddings only through the `Embedder`
//! trait, so the model can be plugged in or replaced without touching
//! indexing or search.
//!
//! STATUS: no real model is wired in yet. The intended model is
//! BAAI/bge-small-en-v1.5 (see docs/MODELS.md for the open verification
//! checklist). Until it is, `load()` reports the model as unavailable and
//! Recall runs keyword + file-name search only. A fake embedder exists for
//! unit tests only (`fake.rs`, compiled under `cfg(test)`); it is never part
//! of the app.

#[cfg(test)]
pub mod fake;

use std::sync::Arc;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("embedding failed: {0}")]
pub struct EmbedError(pub String);

/// A text-embedding model running locally.
pub trait Embedder: Send + Sync {
    /// Identifies the model and version. Stored with every vector, so
    /// switching models makes Recall re-embed instead of mixing vectors.
    fn model_id(&self) -> &str;

    /// Length of every vector this model produces.
    fn dimensions(&self) -> usize;

    /// Embed passages for storage. Returned vectors must be L2-normalised
    /// (length 1), so cosine similarity is a plain dot product.
    fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>;

    /// Embed a search query. Separate from passages because retrieval models
    /// such as BGE may treat queries differently (e.g. an instruction prefix).
    fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError>;
}

/// The embedding model available to this run of Recall, if any.
pub struct EmbeddingModel {
    pub embedder: Option<Arc<dyn Embedder>>,
    /// User-facing reason when `embedder` is `None`.
    pub unavailable_reason: Option<String>,
}

/// Load the local embedding model.
///
/// TODO(Milestone 7 checkpoint): load BAAI/bge-small-en-v1.5 here once its
/// files have been downloaded and verified (docs/MODELS.md). Until then
/// semantic search is honestly reported as unavailable.
pub fn load() -> EmbeddingModel {
    EmbeddingModel {
        embedder: None,
        unavailable_reason: Some(
            "Meaning-based search isn't available yet: the local AI model hasn't been installed. Recall is searching by keywords and file names."
                .into(),
        ),
    }
}

/// Cosine similarity of two L2-normalised vectors (their dot product).
pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Scale a vector to length 1. Leaves an all-zero vector unchanged.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "for the BGE embedder (Milestone 7 checkpoint); used by tests today"
    )
)]
pub fn normalize(vector: &mut [f32]) {
    let length = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if length > 0.0 {
        vector.iter_mut().for_each(|x| *x /= length);
    }
}

/// Vectors are stored in SQLite as little-endian `f32` bytes.
pub fn to_bytes(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn from_bytes(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_round_trip() {
        let vector = vec![0.5, -1.25, 3.0e-7, f32::MAX];
        assert_eq!(from_bytes(&to_bytes(&vector)), vector);
    }

    #[test]
    fn normalized_vectors_have_similarity_one_with_themselves() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        assert_eq!(v, vec![0.6, 0.8]);
        assert!((similarity(&v, &v) - 1.0).abs() < 1e-6);
        let mut zero = vec![0.0, 0.0];
        normalize(&mut zero);
        assert_eq!(zero, vec![0.0, 0.0]);
    }

    #[test]
    fn no_model_is_loaded_until_bge_is_verified() {
        let model = load();
        assert!(model.embedder.is_none());
        assert!(model.unavailable_reason.is_some());
    }
}
