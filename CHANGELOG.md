# Changelog

All notable changes to Recall are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Recall will
use [Semantic Versioning](https://semver.org/) once it has releases.

## [Unreleased]

There are no releases yet. This section describes Recall as it stands.

### Added

- Desktop app (Tauri 2, Rust and React) that indexes folders you choose and
  keeps them up to date with a file watcher. Unchanged files are skipped
  using size, modification time and a SHA-256 fingerprint.
- Text extraction from PDF (page by page), Word (.docx), plain text (UTF-8
  and UTF-16) and Markdown files.
- Local OCR for PNG, JPG and WEBP images with the ocrs models.
- Search by meaning with BAAI/bge-small-en-v1.5, running locally on the
  rten runtime, combined with keyword search (SQLite FTS5) and file-name
  matching. Results say why they matched.
- Evidence view that shows the matched passage highlighted inside its page,
  and a button to open the original file.
- Privacy page with facts measured from the running app.
- `npm run download-models`, which fetches the model files at pinned
  versions and checks their SHA-256 checksums. Installers bundle the models,
  so the app itself never downloads anything.
- Privacy audit (`npm run audit:privacy`) and CI that runs the tests,
  lints, the audit and the real-model tests with networking switched off,
  and builds a Windows installer.
- Mock mode (`npm run dev:mock`) for working on the interface without Rust
  or models.

### Changed

- The app identifier is now `io.github.akanjiolayinka.recall` (it was
  `ai.recall.desktop`). If you installed a test build before this change,
  Recall will start with an empty index and re-read your folders once. You
  can delete the old `ai.recall.desktop` folder from your app-data
  directory.
- The minimum Rust version is now stated correctly as 1.94, which the rten
  runtime requires.
- The privacy audit now also catches socket code in Rust, `EventSource`
  and hard-coded web addresses in the frontend, network or analytics
  packages, and network permissions on the app window.
- Sample data uses clearly fictional business names.

### Known limitations

- Meaning-based search and keyword stemming are English-only.
- PDF pages without a text layer (scans) are detected but not OCR'd.
- macOS hasn't been tested, and there are no published installers yet.
