# AI models

Recall runs AI models **only on this computer**. No text is ever sent to an
online AI service. Hackathon rule: every model must have **at most 500
million parameters**, verified — not assumed.

## Current status

| Purpose | Intended model | Status |
| --- | --- | --- |
| Embeddings (meaning-based search) | `BAAI/bge-small-en-v1.5` | **Not installed, not verified.** Code is ready to plug it in; see checkpoint below. |
| OCR (text in images) | `ocrs` text-detection + text-recognition models | **Working, verified** (licence of the model weights still to confirm — see below) |
| Optional local LLM | e.g. `Qwen2.5-0.5B-Instruct` (to be verified) | Not started; optional |

Until the embedding model is installed, Recall searches by **keywords and
file names only** and says so on the Search page. Nothing in the app
pretends to use AI that isn't there.

Install the verified model files with `npm run download-models`
(`scripts/download-models.mjs`, Node.js only). It checks every file's SHA-256
and refuses files that don't match. Files go to `models/` (not in Git).
Recall also looks in `$RECALL_MODELS_DIR` if that is set.

## OCR: ocrs

Chosen over PaddleOCR because PaddleOCR's model hosts (Baidu, Hugging Face)
were unreachable from the development environment, and because ocrs is pure
Rust (no native libraries to install or ship on Windows).

| | |
| --- | --- |
| Engine | [`ocrs`](https://crates.io/crates/ocrs) 0.13.1 with `rten` 0.26 (pure Rust); licence **MIT OR Apache-2.0** (from the crate metadata) |
| Models | `text-detection.onnx`, `text-recognition.onnx`, from the URLs in ocrs-cli 0.13.1's source (`ocrs-models.s3-accelerate.amazonaws.com`) |
| Parameters | **620,538** (detection) + **2,426,494** (recognition) = **3,047,032** — counted by summing every weight tensor in the ONNX files with the `onnx` Python package |
| SHA-256 | detection `a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe`<br>recognition `86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5` |
| Training data | Google's HierText dataset (stated in ocrs-cli's source) |
| Weights licence | **Not yet verified.** The ocrs project README on GitHub states it; it could not be read from the development environment. Expected to be CC BY-SA 4.0 (the HierText licence) — confirm before release. |
| Tested | Reads `test-data/images/Headphones receipt.png` exactly (`cargo test --release -- --ignored`, with the models installed); ~0.35 s per image in a release build |
| Shipping | Bundled into installers as resources (`tauri.conf.json`); tested in an installed Linux `.deb` with no network access |

Not supported yet: scanned PDFs (PDF pages would need rendering to images
first) and word coordinates for highlighting inside images.

## Embeddings: what is built

- `src-tauri/src/embeddings/mod.rs` — the `Embedder` trait every model
  implements (`embed_passages`, `embed_query`, `model_id`, `dimensions`), and
  `load()`, which currently reports the model as unavailable.
- `src-tauri/src/indexing/embed.rs` — embeds passages in batches during
  indexing; re-embeds automatically if `model_id` changes.
- `src-tauri/src/database/embeddings.rs` — stores vectors (little-endian
  `f32`) tagged with the model id; nearest-neighbour lookup (measured: ~56 ms
  over 50,000 × 384-dim vectors, release build).
- `src-tauri/src/search/` — hybrid ranking: 65% meaning, 25% keywords, 10%
  file name/title; without a model, 50% keywords and 50% file name/title.
- Tests use a **fake, test-only** embedder (`embeddings/fake.rs`, compiled
  only under `cfg(test)`) and a hand-made "housing" embedder in
  `search/mod.rs`. They test the plumbing only, **not** search quality.

Runtime chosen (not yet exercised with the real model): `candle` 0.11
(pure-Rust inference, MIT/Apache-2.0) with `tokenizers` 0.22. Note:
candle-core 0.11 requires `tokenizers` with the `onig` feature, which
compiles a small C regex library (Oniguruma); this needs the MSVC C/C++ build
tools on Windows, which Tauri already requires.

## Checkpoint: Milestone 7 (blocked on network access to huggingface.co)

Do these in order once `huggingface.co` is reachable. Do not tick a box
without evidence.

- [ ] Download the official files from https://huggingface.co/BAAI/bge-small-en-v1.5
      at a pinned commit: `config.json`, `tokenizer.json`, `model.safetensors`.
      Record the commit hash and each file's SHA-256 here.
- [ ] Verify the parameter count by summing tensor sizes in
      `model.safetensors` (expected around 33 million; must be ≤ 500 million).
      Record the exact number here.
- [ ] Verify the licence from the official repository (model card / LICENSE)
      and record it here.
- [ ] Implement `BgeEmbedder` (candle `BertModel`, CLS pooling, L2
      normalisation, query instruction prefix as documented on the model card)
      and return it from `embeddings::load()`.
- [ ] Check output against a reference: compare vectors for a few sentences
      with the official `sentence-transformers` implementation (cosine ≥ 0.99).
- [ ] Test real semantic similarity on `test-data/`: e.g. "apartment notes"
      should find the tenancy agreement and moving-out checklist; unrelated
      queries should not.
- [ ] Calibrate `search::semantic::MIN_SIMILARITY` (currently a placeholder,
      0.5) and re-check the hybrid weights and result labels.
- [ ] Write `npm run download-models` (Node, no Python needed): downloads the
      pinned files into `models/`, verifies SHA-256, explains errors.
- [ ] Decide how models ship in the installer (Tauri bundle resources vs.
      first-run download) and test offline.

Re-test after the checkpoint: Phase 8 (meaning search) and Phase 9 (hybrid
ranking) in the real app.
