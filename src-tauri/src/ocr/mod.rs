//! Reading text in images (OCR) with the local `ocrs` engine.
//!
//! Models: ocrs `text-detection.onnx` (620,538 parameters) and
//! `text-recognition.onnx` (2,426,494 parameters), counted from the files.
//! See docs/MODELS.md for sources, checksums and licence status.

use std::path::{Path, PathBuf};

use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

use crate::extract::{ExtractError, Extracted, Page};

/// Folder inside a models directory holding the ocrs files.
const MODEL_SUBDIR: &str = "ocrs";
const DETECTION_MODEL: &str = "text-detection.onnx";
const RECOGNITION_MODEL: &str = "text-recognition.onnx";

/// Images larger than this are skipped rather than decoded into memory.
const MAX_IMAGE_BYTES: u64 = 40 * 1024 * 1024;

pub struct Ocr {
    engine: OcrEngine,
}

/// The OCR engine available to this run of Recall, if any.
pub struct OcrModel {
    pub engine: Option<std::sync::Arc<Ocr>>,
    /// User-facing reason when `engine` is `None`.
    pub unavailable_reason: Option<String>,
}

impl OcrModel {
    /// Load the OCR models from the first folder that has them.
    pub fn load(model_dirs: &[PathBuf]) -> Self {
        let Some(dir) = model_dirs
            .iter()
            .map(|dir| dir.join(MODEL_SUBDIR))
            .find(|dir| {
                dir.join(DETECTION_MODEL).is_file() && dir.join(RECOGNITION_MODEL).is_file()
            })
        else {
            return Self {
                engine: None,
                unavailable_reason: Some(
                    "Text in images can't be read yet: the OCR model files aren't installed. Run `npm run download-models`."
                        .into(),
                ),
            };
        };
        match Ocr::load(&dir) {
            Ok(ocr) => Self {
                engine: Some(std::sync::Arc::new(ocr)),
                unavailable_reason: None,
            },
            Err(err) => {
                eprintln!(
                    "recall: could not load OCR models from {}: {err}",
                    dir.display()
                );
                Self {
                    engine: None,
                    unavailable_reason: Some(
                        "Recall couldn't load its OCR model files. Run `npm run download-models` again to replace them."
                            .into(),
                    ),
                }
            }
        }
    }
}

impl Ocr {
    pub fn load(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(Model::load_file(dir.join(DETECTION_MODEL))?),
            recognition_model: Some(Model::load_file(dir.join(RECOGNITION_MODEL))?),
            ..Default::default()
        })?;
        Ok(Self { engine })
    }

    /// Read the text in an image file, one line per detected text line.
    pub fn read_image(&self, path: &Path) -> Result<Extracted, ExtractError> {
        let size = std::fs::metadata(path)
            .map_err(|e| ExtractError::Io(e.to_string()))?
            .len();
        if size > MAX_IMAGE_BYTES {
            return Err(ExtractError::TooLarge);
        }
        let image = image::open(path)
            .map_err(|e| ExtractError::Malformed(e.to_string()))?
            .into_rgb8();
        let source = ImageSource::from_bytes(image.as_raw(), image.dimensions())
            .map_err(|e| ExtractError::Malformed(e.to_string()))?;
        let input = self
            .engine
            .prepare_input(source)
            .map_err(|e| ExtractError::Malformed(e.to_string()))?;
        let text = self
            .engine
            .get_text(&input)
            .map_err(|e| ExtractError::Malformed(e.to_string()))?;
        Ok(Extracted {
            pages: vec![Page {
                number: None,
                text: text.trim().to_string(),
            }],
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the real model files, so it's skipped by default. Run with:
    /// `RECALL_MODELS_DIR=../models cargo test --release -- --ignored`
    #[test]
    #[ignore = "needs the OCR model files (npm run download-models)"]
    fn reads_the_synthetic_receipt() {
        let dir = PathBuf::from(
            std::env::var("RECALL_MODELS_DIR").unwrap_or_else(|_| "../models".into()),
        );
        let ocr = Ocr::load(&dir.join(MODEL_SUBDIR)).expect("OCR models installed");
        let receipt = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../test-data/images/Headphones receipt.png");
        let text = &ocr.read_image(&receipt).unwrap().pages[0].text;
        assert!(text.contains("SOUNDWAVE ELECTRONICS"), "{text}");
        assert!(text.contains("Wireless headphones NGN 45,000"), "{text}");
        assert!(text.contains("TOTAL NGN 50,000"), "{text}");
    }

    #[test]
    fn reports_missing_models_plainly() {
        let model = OcrModel::load(&[PathBuf::from("/nonexistent")]);
        assert!(model.engine.is_none());
        assert!(model
            .unavailable_reason
            .unwrap()
            .contains("npm run download-models"));
    }
}
