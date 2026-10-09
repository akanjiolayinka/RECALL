# AI models

Recall runs AI models **only on this computer**. No text is ever sent to an
online AI service. Hackathon rule: every model must have **at most 500
million parameters**, verified — not assumed.

## Current status

| Purpose | Model | Parameters | Status |
| --- | --- | --- | --- |
| Embeddings (meaning-based search) | `BAAI/bge-small-en-v1.5` | 33,212,160 | **Working, verified** |
| OCR (text in images) | `ocrs` text-detection + text-recognition models | 3,047,032 | **Working, verified** (licence situation recorded below) |
| Optional local LLM | e.g. `Qwen2.5-0.5B-Instruct` (to be verified) | — | Not started; optional |

If a model's files are missing, its feature reports itself unavailable with
a reason and everything else keeps working (without the embedding model,
Recall searches by keywords and file names only, and says so). Nothing in the
app pretends to use AI that isn't there.

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
| Weights licence | **No separate licence is stated for the model weights.** The ocrs-models README (checked by the project owner on GitHub) says the models are trained exclusively on open datasets with non-restrictive licences, currently HierText, licensed **CC BY-SA 4.0**. The ocrs engine code is MIT OR Apache-2.0 (LICENSE files confirmed). Recall ships the weights unmodified and credits ocrs and HierText (README, "Credits"). |
| Tested | Reads `test-data/images/Headphones receipt.png` exactly (`cargo test --release -- --ignored`, with the models installed); ~0.35 s per image in a release build |
| Shipping | Bundled into installers as resources (`tauri.conf.json`); tested in an installed Linux `.deb` with no network access |

Not supported yet: scanned PDFs (PDF pages would need rendering to images
first) and word coordinates for highlighting inside images.

## Embeddings: BAAI/bge-small-en-v1.5

Verified on GitHub Actions runners, which can reach Hugging Face (the
development environment can't). Every number below comes from the CI log of
the `checks` job in `.github/workflows/build.yml`, which repeats these checks
on every push.

| | |
| --- | --- |
| Source | https://huggingface.co/BAAI/bge-small-en-v1.5, revision `5c38ec7c405ec4b44b94cc5a9bb96e735b38267a` (pinned in `scripts/download-models.mjs`) |
| Files | `onnx/model.onnx` (133,093,490 bytes, ONNX opset 11, published by BAAI in the model repository), `tokenizer.json` (711,396 bytes) |
| SHA-256 | model.onnx `828e1496d7fabb79cfa4dcd84fa38625c0d3d21da474a00f08db0f559940cf35`<br>tokenizer.json `d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66` |
| Parameters | **33,212,160**, summing every weight tensor (initializer) in `model.onnx` with the `onnx` Python package. The rten runtime reports 33,213,055 (it also counts small constant tensors in the graph). Either way far below 500 million. |
| Architecture | BERT (`config.json`): 12 layers, hidden size 384, 12 attention heads, vocabulary 30,522, max 512 tokens |
| Licence | **MIT**, as declared in the model card metadata (`license: mit`) of the official repository |
| Runtime | `rten` 0.26 (the same pure-Rust runtime as OCR) and `rten-text` 0.26 for WordPiece tokenization from `tokenizer.json`; both MIT OR Apache-2.0. Pure Rust, no network code. |
| How it's used | `src-tauri/src/embeddings/bge.rs`: the `[CLS]` token's output vector, L2-normalised, as the model card describes; the exact agreement with the official implementation (next row) confirms it. Queries get the model card's instruction prefix "Represent this sentence for searching relevant passages: "; passages don't. Inputs longer than 512 tokens are cut off (Recall's passages are ~1,000 characters, well under that). |
| Correctness | Vectors match the official implementation (sentence-transformers, same revision) with cosine agreement **1.000000** on all 5 test texts (`matches_the_reference_implementation`; reference vectors from `scripts/bge_reference.py`). |
| Search quality | On `test-data/`, searches sharing no words with the right file find it first: "what is the price of the allotment" → garden proposal, "when do I get my money back after leaving the apartment" → tenancy agreement and moving-out checklist, "promotion expenses" → marketing plan, "groceries to buy" → shopping list, "headset purchase" → headphones receipt (via OCR). Unrelated queries ("quantum physics lecture", "recipe for chocolate cake") find nothing. Test: `real_model_finds_test_files_by_meaning`. |
| Threshold | `MIN_SIMILARITY` = 0.55, from the same test: the right file scored 0.57–0.77, other files 0.41–0.55, unrelated queries at most 0.46. Tested on a small synthetic set only; it may need tuning on real libraries. |
| Speed | ~60 ms per text on a GitHub Actions runner; `test-data/` (13 passages, including reading the files and OCR) indexed in 2.9 s |
| Shipping | Downloaded and checksum-verified by `npm run download-models`, bundled into installers as resources (`tauri.conf.json`), so an installed Recall needs no download |

### How the rest of Recall uses it

- `src-tauri/src/embeddings/mod.rs`: the `Embedder` trait every model
  implements, and `load()`, which finds the model files.
- `src-tauri/src/indexing/embed.rs`: embeds passages in batches during
  indexing; re-embeds automatically if `model_id` changes.
- `src-tauri/src/database/embeddings.rs`: stores vectors (little-endian
  `f32`) tagged with the model id, plus a nearest-neighbour lookup (measured:
  ~56 ms over 50,000 × 384-dim vectors, release build).
- `src-tauri/src/search/`: hybrid ranking, 65% meaning, 25% keywords and 10%
  file name/title; without a model, 50% keywords and 50% file name/title.
- Unit tests use a **fake, test-only** embedder (`embeddings/fake.rs`,
  compiled only under `cfg(test)`) to test the plumbing; search quality is
  tested with the real model (above).

### Re-running the checks locally

```bash
npm run download-models
pip install sentence-transformers
python scripts/bge_reference.py 5c38ec7c405ec4b44b94cc5a9bb96e735b38267a models/bge-small-en-v1.5/reference.json
cd src-tauri && cargo test -- --ignored --nocapture
```

### Changing the embedding model

Implement `Embedder` for it, return it from `embeddings::load()`, give it a
new `model_id` (stored vectors are then re-made automatically), record its
verification here the same way, and re-calibrate `MIN_SIMILARITY` with
`real_model_finds_test_files_by_meaning`.
