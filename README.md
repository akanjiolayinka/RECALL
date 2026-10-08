This project was submitted to the ryze.ai hackathon by Olayinka Akanji.

# Recall

**Your private AI memory. Everything searchable. Nothing uploaded.**

Recall is a local-first desktop app for searching the files on your computer by
what you remember about them, not by filename. All AI runs on your device.

> **Status: early development (Milestone 5 of 15).** You can choose folders;
> Recall finds supported files, reads the text of PDF, Word, text and Markdown
> files, and stores it in a local SQLite database. **Search is not
> implemented yet**, and images are not read yet (OCR comes later).
> This README only describes what currently works; it will grow as features land.

## What works today

| Feature | Status |
| --- | --- |
| Desktop app launches (Tauri + React) | Working |
| Frontend talks to the Rust backend | Working |
| Development mock mode for UI work | Working |
| Choose folders in Library (system folder picker) | Working |
| Find supported files (PDF, DOCX, TXT, MD, PNG, JPG, WEBP) with size, dates and a SHA-256 fingerprint | Working |
| Remember folders and files between launches (local SQLite database) | Working |
| Re-check folders at startup and on Rescan; unchanged files aren't re-read | Working |
| Read text from TXT, Markdown, PDF (with page numbers) and DOCX (with title/author); split into passages | Working |
| View the text Recall extracted from a file | Working |
| Clear messages for files that can't be read (damaged, password-protected, scanned PDFs, over 100 MB) | Working |
| Browse and filter found files by name and type; live scan progress | Working |
| Search (keyword and meaning-based), OCR for images and scanned PDFs, embeddings | Not implemented yet |

## Requirements

- [Node.js](https://nodejs.org/) 20 or newer (developed with 22)
- [Rust](https://rustup.rs/) (stable)
- Tauri's system prerequisites for your OS: <https://v2.tauri.app/start/prerequisites/>

## Run it

```bash
npm install
npm run tauri dev
```

The first run compiles the Rust backend and takes a few minutes.

### Where Recall keeps its data

Recall's index is a single SQLite file, `recall.db`, in the app data folder:

| OS | Location |
| --- | --- |
| Windows | `%APPDATA%\ai.recall.desktop\` |
| macOS | `~/Library/Application Support/ai.recall.desktop/` |
| Linux | `~/.local/share/ai.recall.desktop/` |

It contains file paths, sizes, dates and fingerprints — never copies of your
files. Deleting it resets Recall; your own files are never modified.

### Run the backend tests

```bash
cd src-tauri
cargo test
```

### UI-only mock mode (no Rust needed)

```bash
npm run dev:mock
```

Then open <http://localhost:1420> in a browser. An amber banner marks fake
development data. Mock mode is never included in production builds.

## Project layout

```text
src/                 React + TypeScript frontend
  components/ui/     Reusable UI building blocks (shadcn/ui style)
  components/layout/ App shell: sidebar, page header, placeholders
  pages/             One file per screen
  components/library/ Library screen pieces (folder list, file browser)
  components/indexing/ Scan progress display
  components/evidence/ Viewer for a file's extracted text
  hooks/             React hooks that load data through the API client
  lib/api/           The only place the frontend talks to the backend
src-tauri/           Rust backend (Tauri)
  src/commands/      Commands the frontend can call (thin adapters)
  src/locations/     Rules for which folders can be added (unit-tested)
  src/files/         Finding, describing, fingerprinting and filtering files (unit-tested)
  src/scanning.rs    Runs scans in the background, saves results, reports progress
  src/database/      SQLite storage and numbered schema migrations (unit-tested)
  src/extract/       Text extraction for TXT, MD, PDF, DOCX (unit-tested)
  src/indexing/      Splitting text into searchable passages (unit-tested)
test-data/           Synthetic demo files (see test-data/README.md)
scripts/             Developer scripts (test data generator)
  src/error.rs       The error format every command returns
```
