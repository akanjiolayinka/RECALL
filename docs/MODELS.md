# AI models

Recall runs AI models **only on this computer**. No text is ever sent to an
online AI service. Hackathon rule: every model must have **at most 500
million parameters**, verified — not assumed.

## Current status

| Purpose | Intended model | Status |
| --- | --- | --- |
| Embeddings (meaning-based search) | `BAAI/bge-small-en-v1.5` | **Not installed, not verified.** Code is ready to plug it in; see checkpoint below. |
| OCR (text in images) | small PP-OCR model (to be chosen) | Not started |
| Optional local LLM | e.g. `Qwen2.5-0.5B-Instruct` (to be verified) | Not started; optional |

Until the embedding model is installed, Recall searches by **keywords and
file names only** and says so on the Search page. Nothing in the app
pretends to use AI that isn't there.

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
