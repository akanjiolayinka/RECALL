# Contributing to Recall

Recall is a private, local-first search engine for the files on your
computer: search by what you remember, with every result backed by the
actual text it came from. Nothing is uploaded and all AI runs on the device.
Thanks for helping.

Start with [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (10 minutes). It
explains how the pieces fit and where to make changes.

## Install and run

You need:

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable, 1.82 or newer)
- Tauri's prerequisites for your OS: <https://v2.tauri.app/start/prerequisites/>
  (on Windows: Microsoft C++ Build Tools and WebView2)

```bash
npm install
npm run download-models   # local AI model files (~145 MB), checksum-verified
npm run tauri dev         # runs the desktop app; first build takes a few minutes
```

Add the `test-data/` folder in **Library** to try Recall on synthetic files.

Frontend-only work needs just Node: `npm run dev:mock` and open
<http://localhost:1420> (see [docs/FRONTEND_QUICKSTART.md](docs/FRONTEND_QUICKSTART.md)).

## Test

```bash
npm test                     # frontend unit tests (Vitest)
npm run build                # frontend type-check and build
cd src-tauri
cargo test                   # backend unit tests (fast, no models needed)
cargo clippy --all-targets   # lints; keep it at zero warnings
cargo fmt                    # formatting
cargo test -- --ignored --nocapture   # tests that need the model files (see docs/MODELS.md)
cd ..
npm run audit:privacy        # no network code, no cloud AI, locked-down window
```

## Repository structure

```text
src/                   React + TypeScript frontend (FRONTEND_QUICKSTART.md)
src-tauri/
  src/commands/        Tauri commands: thin adapters, the backend side of the API
  src/locations/       which folders can be added
  src/files/           finding, describing and fingerprinting files
  src/extract/         text from TXT, MD, PDF, DOCX
  src/ocr/             text from images (local ocrs model)
  src/indexing/        passages (chunk), embeddings (embed), the pipeline
  src/embeddings/      the Embedder interface and the local BGE model (bge.rs)
  src/search/          keyword, meaning and file-name search; hybrid ranking
  src/database/        SQLite storage and migrations
  src/scanning.rs      runs the pipeline in the background (Tauri glue)
  src/watching.rs      re-scans folders when files change (Tauri glue)
docs/                  API.md, ARCHITECTURE.md, FRONTEND_QUICKSTART.md, MODELS.md
scripts/               model download, privacy audit, test-data generator,
                       BGE reference vectors (bge_reference.py)
test-data/             synthetic files for tests and demos (never real documents)
models/                downloaded model files (not in Git)
```

## The API boundary

The frontend and backend meet in one place: the API in
[docs/API.md](docs/API.md), defined in `src/lib/api/types.ts` and mirrored
by the DTOs in `src-tauri/src/commands/`.

- The frontend never knows about tables, Rust types or models.
- The backend never knows about components or styling.
- Changing the API means updating, in the same pull request: `types.ts`,
  the Rust DTO and command, `tauri.ts`, `mock.ts` and `docs/API.md`.

If a frontend change seems to need a database change, or a backend
optimisation seems to need many UI changes, stop and reconsider: it
probably belongs on the other side of the boundary.

## Contributing to…

**The frontend.** See [docs/FRONTEND_QUICKSTART.md](docs/FRONTEND_QUICKSTART.md).
Work in mock mode; use the design tokens and `components/ui/`; every
data-loading screen needs loading, empty and error states.

**The backend.** Put logic in plain Rust modules (no Tauri) with unit tests
next to it; keep commands thin. Storage functions take a `&Connection` so
they can be tested in memory (`database::test_connection()`), and
`src/test_support.rs` builds small test libraries. Schema changes are a new
numbered file in `src-tauri/src/database/migrations/`; never edit a released
migration.

**AI models.** Read [docs/MODELS.md](docs/MODELS.md) first. Hard rules:

- Models run **locally only**. No cloud AI APIs, ever.
- Every model must have **at most 500 million parameters**, counted from the
  files, not taken from a web page.
- Record the exact source, version, parameter count, licence and SHA-256 in
  `docs/MODELS.md`, and add the file to `scripts/download-models.mjs`.
- Test against the real model; fakes (`embeddings/fake.rs`) are for testing
  plumbing only.
- A missing model must leave the app working and say so plainly.

## Making a change

1. Find the layer it belongs to (ARCHITECTURE.md, "Where to change things").
2. Make the smallest change that does the job; prefer clear code over clever
   code, and don't add abstractions until there's a real second use.
3. Add or update tests for logic you change.
4. Run the checks under **Test** above.
5. Run the app and try the change, including the unhappy paths.
6. Update docs that describe what you changed (README status table, API.md…).

## Conventions

- **Honesty in the product.** Never show invented data: no fake results,
  snippets, page numbers, counters or AI claims. If Recall can't stand behind
  something, it says so.
- **User-facing messages** are plain language with no codes or jargon;
  technical detail goes to the log (`eprintln!`), and logs never contain
  document text.
- **Rust:** `cargo fmt`, zero `cargo clippy` warnings, `thiserror` for error
  types, no `unwrap()` outside tests (except where documented as impossible).
- **TypeScript:** strict mode, `@/` imports, types for anything crossing the
  API.
- **Comments** explain *why*, not what the code obviously does.
- **Dependencies:** add one only when it clearly earns its place; prefer
  pure-Rust crates (they build reliably on Windows) and check their licence.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/) with a
clear summary and a body explaining why:

```text
feat: add local OCR for images
fix: refresh search results after a scan
docs: describe the evidence API
```

Avoid messages like `changes`, `stuff` or `final2`. Never commit secrets,
real personal documents, model files or build output.

## Pull requests

- One focused change per pull request, with a description of what changed,
  why, and how you tested it (commands run, screens checked).
- All checks above pass; `npm run audit:privacy` passes.
- API changes update both sides, the mock and `docs/API.md` together.
- New models come with their verification record in `docs/MODELS.md`.
