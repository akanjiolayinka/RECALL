//! TEST-ONLY fake embedder. Compiled only under `cfg(test)`; never in the app.
//!
//! It has no understanding of meaning: it hashes each word into one of a few
//! buckets, so texts that share words get similar vectors. That is enough to
//! test the plumbing around embeddings (storage, re-embedding, vector search,
//! hybrid ranking). Any search quality seen with it says nothing about how
//! the real model will behave.

use super::{normalize, EmbedError, Embedder};

pub const FAKE_DIMENSIONS: usize = 64;

pub struct FakeEmbedder {
    model_id: String,
}

impl FakeEmbedder {
    pub fn new(model_id: &str) -> Self {
        Self {
            model_id: model_id.into(),
        }
    }

    fn embed(text: &str) -> Vec<f32> {
        let mut vector = vec![0.0; FAKE_DIMENSIONS];
        for word in text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
        {
            let word = word.to_lowercase();
            // Crude singular form, so "budgets" and "budget" land together.
            let word = word.strip_suffix('s').unwrap_or(&word);
            // FNV-1a: a simple, stable hash.
            let hash = word.bytes().fold(0xcbf29ce484222325u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
            });
            vector[(hash % FAKE_DIMENSIONS as u64) as usize] += 1.0;
        }
        normalize(&mut vector);
        vector
    }
}

impl Embedder for FakeEmbedder {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn dimensions(&self) -> usize {
        FAKE_DIMENSIONS
    }

    fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts.iter().map(|t| Self::embed(t)).collect())
    }

    fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(Self::embed(text))
    }
}
