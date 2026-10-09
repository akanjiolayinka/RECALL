//! Embeddings: turning text into vectors whose closeness reflects meaning.
//!
//! Everything else in Recall talks to embeddings only through the `Embedder`
//! trait, so the model can be plugged in or replaced without touching
//! indexing or search.
//!
//! The model is BAAI/bge-small-en-v1.5 (`bge.rs`, docs/MODELS.md). When its
//! files aren't installed, `load()` says so and Recall runs keyword +
//! file-name search only. A fake embedder exists for unit tests only
//! (`fake.rs`, compiled under `cfg(test)`); it is never part of the app.

pub mod bge;
#[cfg(test)]
pub mod fake;

use std::path::PathBuf;
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

/// Load the embedding model from the first folder that has its files.
pub fn load(model_dirs: &[PathBuf]) -> EmbeddingModel {
    let unavailable = |reason: &str| EmbeddingModel {
        embedder: None,
        unavailable_reason: Some(reason.into()),
    };
    let Some(dir) = model_dirs
        .iter()
        .map(|dir| dir.join(bge::MODEL_SUBDIR))
        .find(|dir| dir.join("model.onnx").is_file() && dir.join("tokenizer.json").is_file())
    else {
        return unavailable(
            "Meaning-based search isn't available: the AI model files aren't installed. Run `npm run download-models`. Recall is searching by keywords and file names.",
        );
    };
    match bge::BgeSmall::load(&dir) {
        Ok(model) => EmbeddingModel {
            embedder: Some(Arc::new(model)),
            unavailable_reason: None,
        },
        Err(err) => {
            eprintln!(
                "recall: could not load the embedding model from {}: {err}",
                dir.display()
            );
            unavailable(
                "Recall couldn't load its meaning-search model files. Run `npm run download-models` again to replace them.",
            )
        }
    }
}

/// Cosine similarity of two L2-normalised vectors (their dot product).
pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Scale a vector to length 1. Leaves an all-zero vector unchanged.
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
    fn reports_missing_model_files_plainly() {
        let model = load(&[PathBuf::from("/nonexistent")]);
        assert!(model.embedder.is_none());
        assert!(model
            .unavailable_reason
            .unwrap()
            .contains("npm run download-models"));
    }
}
