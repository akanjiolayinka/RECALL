//! BAAI/bge-small-en-v1.5 running locally with rten.
//!
//! A BERT-style encoder: text is split into WordPiece tokens, the model
//! produces one vector per token, and BGE uses the first (`[CLS]`) token's
//! vector, L2-normalised, as the text's embedding. Queries get the
//! instruction prefix the model's authors recommend for short queries
//! searching longer passages; passages are embedded as they are.
//!
//! Sources, parameter count, licence and checksums: docs/MODELS.md.

use std::path::Path;

use rten::{Model, NodeId};
use rten_tensor::prelude::*;
use rten_tensor::NdTensor;
use rten_text::tokenizer::EncodeOptions;
use rten_text::Tokenizer;

use super::{normalize, EmbedError, Embedder};

/// Folder inside a models directory holding this model's files.
pub const MODEL_SUBDIR: &str = "bge-small-en-v1.5";
const MODEL_FILE: &str = "model.onnx";
const TOKENIZER_FILE: &str = "tokenizer.json";

const MODEL_ID: &str = "BAAI/bge-small-en-v1.5";
const DIMENSIONS: usize = 384;
/// The model's maximum input length; longer text is cut off.
const MAX_TOKENS: usize = 512;
/// From the model card, for retrieval with short queries.
const QUERY_INSTRUCTION: &str = "Represent this sentence for searching relevant passages: ";

pub struct BgeSmall {
    model: Model,
    /// rten-text's `Tokenizer` can't be shared between threads, so each call
    /// builds one from this JSON. That takes milliseconds; running the model
    /// on a passage takes far longer.
    tokenizer_json: String,
    input_ids: NodeId,
    attention_mask: NodeId,
    token_type_ids: Option<NodeId>,
    output: NodeId,
}

impl BgeSmall {
    /// Load the model from a folder containing `model.onnx` and `tokenizer.json`.
    pub fn load(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let model = Model::load_file(dir.join(MODEL_FILE))?;
        let tokenizer_json = std::fs::read_to_string(dir.join(TOKENIZER_FILE))?;
        Tokenizer::from_json(&tokenizer_json)
            .map_err(|e| format!("unreadable {TOKENIZER_FILE}: {e:?}"))?;
        let input_ids = model.node_id("input_ids")?;
        let attention_mask = model.node_id("attention_mask")?;
        let token_type_ids = model.find_node("token_type_ids");
        // The first output is the per-token hidden state.
        let output = *model
            .output_ids()
            .first()
            .ok_or("the model has no outputs")?;
        Ok(Self {
            model,
            tokenizer_json,
            input_ids,
            attention_mask,
            token_type_ids,
            output,
        })
    }

    /// Weights in the model file, as counted by the runtime.
    #[cfg(test)]
    pub fn parameter_count(&self) -> usize {
        self.model.total_params()
    }

    fn tokenizer(&self) -> Result<Tokenizer, EmbedError> {
        Tokenizer::from_json(&self.tokenizer_json).map_err(|e| EmbedError(format!("{e:?}")))
    }

    fn embed(&self, tokenizer: &Tokenizer, text: &str) -> Result<Vec<f32>, EmbedError> {
        let options = EncodeOptions {
            max_chunk_len: Some(MAX_TOKENS),
            overlap: 0,
        };
        let encoded = tokenizer
            .encode(text, Some(options))
            .map_err(|e| EmbedError(e.to_string()))?;
        let ids: Vec<i32> = encoded.token_ids().iter().map(|&id| id as i32).collect();
        let len = ids.len();
        let input_ids = NdTensor::from_data([1, len], ids);
        let attention_mask = NdTensor::full([1, len], 1i32);
        let token_type_ids = NdTensor::full([1, len], 0i32);

        let mut inputs = vec![
            (self.input_ids, input_ids.view().into()),
            (self.attention_mask, attention_mask.view().into()),
        ];
        if let Some(id) = self.token_type_ids {
            inputs.push((id, token_type_ids.view().into()));
        }
        let [hidden] = self
            .model
            .run_n(inputs, [self.output], None)
            .map_err(|e| EmbedError(e.to_string()))?;
        let hidden: NdTensor<f32, 3> = hidden
            .try_into()
            .map_err(|e| EmbedError(format!("unexpected model output: {e:?}")))?;
        if hidden.size(2) != DIMENSIONS {
            return Err(EmbedError(format!(
                "model output has {} dimensions, expected {DIMENSIONS}",
                hidden.size(2)
            )));
        }
        let mut vector: Vec<f32> = hidden.slice((0, 0)).iter().copied().collect();
        normalize(&mut vector);
        Ok(vector)
    }
}

impl Embedder for BgeSmall {
    fn model_id(&self) -> &str {
        MODEL_ID
    }

    fn dimensions(&self) -> usize {
        DIMENSIONS
    }

    fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let tokenizer = self.tokenizer()?;
        texts
            .iter()
            .map(|text| self.embed(&tokenizer, text))
            .collect()
    }

    fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed(&self.tokenizer()?, &format!("{QUERY_INSTRUCTION}{text}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::similarity;

    fn load_real_model() -> BgeSmall {
        BgeSmall::load(&crate::test_support::real_models_dir().join(MODEL_SUBDIR))
            .expect("BGE model files installed")
    }

    /// Needs the real model files, so it's skipped by default. Run with:
    /// `RECALL_MODELS_DIR=../models cargo test --release -- --ignored`
    #[test]
    #[ignore = "needs the BGE model files"]
    fn has_at_most_500_million_parameters() {
        let model = load_real_model();
        let params = model.parameter_count();
        println!("BGE parameters counted by rten: {params}");
        assert!(params <= 500_000_000, "{params}");
    }

    #[test]
    #[ignore = "needs the BGE model files"]
    fn related_texts_are_closer_than_unrelated_ones() {
        let model = load_real_model();
        let query = model
            .embed_query("how much money does the garden project need")
            .unwrap();
        let passages = model
            .embed_passages(&[
                "The estimated cost of the community garden is NGN 2,500,000, covering tools, seeds and fencing.",
                "The tenant must give one month's notice before moving out of the flat.",
                "Buy milk, eggs and bread on the way home.",
            ])
            .unwrap();
        for vector in &passages {
            assert_eq!(vector.len(), DIMENSIONS);
            let length: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((length - 1.0).abs() < 1e-4);
        }
        let scores: Vec<f32> = passages.iter().map(|p| similarity(&query, p)).collect();
        println!("similarity to budget / notice / shopping passages: {scores:?}");
        assert!(scores[0] > scores[1] && scores[0] > scores[2], "{scores:?}");
        // The same text always gives the same vector.
        let again = model
            .embed_passages(&["Buy milk, eggs and bread on the way home."])
            .unwrap();
        assert!((similarity(&again[0], &passages[2]) - 1.0).abs() < 1e-5);
    }

    /// Compares with vectors from the official implementation, written by
    /// `scripts/bge_reference.py` to `<models>/bge-small-en-v1.5/reference.json`.
    #[test]
    #[ignore = "needs the BGE model files and reference vectors"]
    fn matches_the_reference_implementation() {
        #[derive(serde::Deserialize)]
        struct Reference {
            text: String,
            query: bool,
            vector: Vec<f32>,
        }
        let path = crate::test_support::real_models_dir()
            .join(MODEL_SUBDIR)
            .join("reference.json");
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("run scripts/bge_reference.py to create {path:?}"));
        let references: Vec<Reference> = serde_json::from_str(&json).unwrap();
        let model = load_real_model();
        for reference in &references {
            let vector = if reference.query {
                model.embed_query(&reference.text).unwrap()
            } else {
                model.embed_passages(&[&reference.text]).unwrap().remove(0)
            };
            let agreement = similarity(&vector, &reference.vector);
            println!(
                "{agreement:.6} agreement with the reference for {:?}",
                reference.text
            );
            assert!(agreement >= 0.99, "{agreement} for {:?}", reference.text);
        }
    }

    #[test]
    #[ignore = "needs the BGE model files"]
    fn empty_and_very_long_texts_still_embed() {
        let model = load_real_model();
        let long = "word ".repeat(5_000);
        let vectors = model.embed_passages(&["", &long]).unwrap();
        assert!(vectors.iter().all(|v| v.len() == DIMENSIONS));
    }
}
