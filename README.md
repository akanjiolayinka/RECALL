This project was submitted to the ryze.ai hackathon by Olayinka Akanji.

# Recall

**Your private AI memory. Everything searchable. Nothing uploaded.**

Recall is a desktop app that lets you search the files on your computer the
way you search your memory. Instead of remembering a file's name or folder,
describe what you remember about it — *"the document about my project
budget"*, *"my headphone receipt"* — and Recall finds it and shows you the
exact passage that proves it.

Everything happens on your computer: indexing, text recognition, search and
the database. Recall contains no internet client and no cloud AI.

> **Status.** All core features work: folders, indexing of documents and
> images, meaning-based search, keyword and file-name search, evidence, file
> watching and the privacy dashboard. Meaning-based search is new: it is
> verified by automated tests with the real model, but hasn't been tried in
> the installed app yet. See [What works](#what-works).

## How it works

1. **Add a folder** in *Library*. Recall finds the PDFs, Word documents,
   text and Markdown files and images (PNG, JPG, WEBP) inside it.
2. **Recall reads them locally**: text from documents (with page numbers for
   PDFs), and text inside images with a small on-device OCR model. Text is
   split into passages and stored in a local SQLite index.
3. **Search** by what you remember. A small local AI model compares the
   *meaning* of your search with every passage, so "groceries to buy" finds
   your shopping list. Recall combines that with matching words and file
   names, ranks files, and explains why each one matched.
4. **Check the source.** *Show source* displays the matching passage
   highlighted inside its page. *The AI finds it. The source proves it.*
5. **It stays up to date.** Recall watches your folders and re-indexes only
   what changed.

## What works

| Feature | Status |
| --- | --- |
| Add/remove folders with the system folder picker | Working |
| Find supported files: PDF, DOCX, TXT, MD, PNG, JPG/JPEG, WEBP | Working |
| Read document text (PDF with page numbers, DOCX with title/author, UTF-8/UTF-16 text) | Working |
| Read text in images with local OCR (ocrs, ~3 million parameters) | Working |
| Keyword search (SQLite FTS5, English word stems) with highlighted snippets and page numbers | Working |
| File-name and title matching (finds images by name) | Working |
| Meaning-based search (local embeddings, BAAI/bge-small-en-v1.5) | Working in automated tests with the real model; first app test pending |
| Hybrid ranking (meaning + keywords + file names) with plain-language "why it matched" reasons | Working |
| Evidence viewer: the passage highlighted within its page; *Open original* | Working |
| Incremental indexing: unchanged files are never re-read | Working |
| File watching: new, changed and deleted files picked up automatically | Working |
| Privacy dashboard with measured facts | Working |
| Privacy audit (`npm run audit:privacy`) | Passing |
| Works with no internet connection (installed app) | Tested on Linux |
| OCR for scanned PDFs (pages without a text layer) | Not supported yet |
| Optional local LLM ("Ask Recall") | Not started (optional) |

## Privacy

- **No network code.** The Windows, macOS and Linux builds contain no HTTP,
  TLS or WebSocket client, and the app window is restricted by a Content
  Security Policy to talking to Recall itself.
- **No cloud AI.** OCR and meaning-based search run in-process on local
  model files.
- **Your files are only read**, never changed, moved, copied or uploaded.
- **One local index**, `recall.db`, in your app-data folder
  (`%APPDATA%\ai.recall.desktop\` on Windows). Delete it to reset Recall.
- **Verified, not just promised:** `npm run audit:privacy` checks all of the
  above and fails if it ever changes. The in-app *Privacy* page shows only
  values measured from the running app.

## AI models

| Purpose | Model | Parameters | Status |
| --- | --- | --- | --- |
| Text in images (OCR) | ocrs `text-detection` + `text-recognition` | 620,538 + 2,426,494 (counted from the files) | In use |
| Meaning-based search | BAAI/bge-small-en-v1.5 | 33,212,160 (counted from the file) | In use |

All models must have at most 500 million parameters and run locally.
Sources, checksums, licences and what is still to verify are in
[docs/MODELS.md](docs/MODELS.md).

## Download a test build (Windows)

Every push to GitHub builds a Windows installer automatically
([`.github/workflows/build.yml`](.github/workflows/build.yml)):

1. On GitHub, open the repository's **Actions** tab and click the latest
   **Build** run with a green tick.
2. Under **Artifacts**, download **Recall-Windows-installer** (a zip).
3. Unzip it and run `Recall_…_x64-setup.exe`.

The installers are not code-signed, so Windows SmartScreen may warn about an
unknown publisher; choose **More info → Run anyway**. Artifacts are kept for
14 days.

## Get started (development)

You need [Node.js](https://nodejs.org/) 20+, [Rust](https://rustup.rs/)
1.82+ and Tauri's prerequisites for your OS
(<https://v2.tauri.app/start/prerequisites/>; on Windows: Microsoft C++
Build Tools and WebView2).

```bash
npm install
npm run download-models   # AI model files (~145 MB) into models/, checksums verified
npm run tauri dev         # first build takes a few minutes
```

Then in the app: **Library → Add folder** and choose `test-data/` (synthetic
sample files) or your own folder, and search from the **Search** page
(**Ctrl+K** / **⌘K** from anywhere).

Build an installer:

```bash
npm run build:app         # downloads/verifies models, then `tauri build`
```

The AI models are bundled into the installer, so an installed Recall needs
no downloads (the embedding model makes it about 130 MB larger). Tested so
far, before meaning-based search was added: the Linux `.deb` (18 MB) installed and run from
a fresh profile **with no network access at all** — adding a folder,
indexing, OCR, search and evidence all worked, and tracing the app process
showed no connection attempts. The Windows installer built by GitHub
Actions has been installed and tested on Windows (adding a folder, indexing,
search and evidence). The macOS installer has not been tested yet.

Frontend-only work, without Rust: `npm run dev:mock` and open
<http://localhost:1420>.

## Tests and checks

```bash
npm test                          # frontend unit tests
npm run build                     # frontend type-check + build
cd src-tauri && cargo test        # backend unit tests
cargo clippy --all-targets        # zero warnings expected
cd .. && npm run audit:privacy    # privacy audit
```

## Documentation

| Document | For |
| --- | --- |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | How Recall works, in 10 minutes |
| [docs/API.md](docs/API.md) | The frontend ↔ backend contract |
| [docs/FRONTEND_QUICKSTART.md](docs/FRONTEND_QUICKSTART.md) | Working on the UI |
| [docs/MODELS.md](docs/MODELS.md) | AI models, verification, checklist |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Setting up, conventions, pull requests |
| [test-data/README.md](test-data/README.md) | The synthetic sample files |

## Built with

Tauri 2 · Rust · React 19 · TypeScript · Vite · Tailwind CSS · shadcn/ui ·
TanStack Query · SQLite (rusqlite, FTS5) · ocrs/rten · pdf-extract ·
notify.

## Credits

- **Meaning-based search:** [BAAI/bge-small-en-v1.5](https://huggingface.co/BAAI/bge-small-en-v1.5)
  by the Beijing Academy of Artificial Intelligence (MIT), run with
  [rten](https://github.com/robertknight/rten) and rten-text by Robert Knight
  (MIT OR Apache-2.0). Recall uses the published model unmodified.
- **OCR:** [ocrs](https://github.com/robertknight/ocrs) by Robert Knight and
  the Ocrs project contributors (MIT OR Apache-2.0). Its models are trained on
  Google's [HierText](https://github.com/google-research-datasets/hiertext)
  dataset (CC BY-SA 4.0). Recall uses the published models unmodified.

## Limitations

- Meaning-based search uses an English model; other languages match by
  keywords and file names only. Its thresholds were tuned on 24
  synthetic documents and 34 searches (docs/MODELS.md) and may need
  adjusting on real libraries.
- The first indexing of a large folder takes a while: each passage is run
  through the model once (tens of milliseconds each).
- Scanned PDFs (no text layer) are detected and reported, not OCR'd.
- Keyword stemming is English-only; file-name matching is case-insensitive
  for ASCII letters only.
- Very large folders are fully re-walked on each change; only changed files
  are re-read.

## Licence

Recall is released under the [MIT License](LICENSE). Third-party components
keep their own licences (see [docs/MODELS.md](docs/MODELS.md) for the models).
