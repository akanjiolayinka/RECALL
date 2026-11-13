# Good first issues

Self-contained tasks, grouped by how much of Recall you need to know. Each
says why it matters, where to start and what "done" looks like. Before you
start, check the issue tracker in case someone's already on it, and leave a
comment so others know. Bigger directions are in [ROADMAP.md](../ROADMAP.md).

The areas are frontend, Rust backend, search quality, packaging and docs.

## Beginner

### Tests for the scan progress messages (frontend)

**Why.** The Library and Indexing pages describe each scan in words ("Reading
3 of 12 documents") using small pure functions, and none of them are tested.
A wrong message is easy to miss by eye.

**Where.** `describeScan`, `describeCounts`, `isScanning` and `scanPercent`
in `src/components/indexing/ScanProgress.tsx`. Look at
`src/lib/format.test.ts` for the style the existing tests use.

**Done when** a new `ScanProgress.test.ts` covers each scan state, including
empty folders and failed scans, and `npm test` passes.

### Remove leftover build-plan references from comments (Rust backend)

**Why.** A few comments mention "Milestone 5" or "Milestone 17" from the
plan Recall was first built to. They mean nothing to new readers.

**Where.** `src-tauri/src/files/mod.rs` and `src-tauri/src/models.rs`. Leave
`src-tauri/src/database/migrations/0001_initial.sql` alone: released
migrations are never edited.

**Done when** the comments describe what the code does without the
milestone numbers, and `cargo fmt --check` and `cargo clippy` pass.

### Update GitHub Actions versions (packaging)

**Why.** CI logs warn that several actions (checkout, setup-node,
setup-python) still target Node.js 20, which GitHub is retiring.

**Where.** `.github/workflows/build.yml`. Check each action's release notes
for the first version that runs on Node.js 24, and for anything else that
changed.

**Done when** CI passes without the deprecation warning.

### Write down how Recall builds on macOS (docs)

**Why.** Nobody has tried Recall on a Mac yet. If you have one, just
following the setup steps and writing down what happens is very useful.

**Where.** Follow [CONTRIBUTING.md](../CONTRIBUTING.md), then update the
macOS section of [DEVELOPMENT.md](DEVELOPMENT.md).

**Done when** the macOS section reflects what really happens, and anything
that broke has an issue.

## Intermediate

### Move through search results with the keyboard (frontend)

**Why.** You can jump to search with Ctrl+K, but then you need the mouse or
a lot of tabbing to reach results.

**Where.** `src/pages/SearchPage.tsx`, `src/components/search/SearchResultCard.tsx`,
and `src/lib/shortcuts.ts` for shortcut handling (it has tests in
`shortcuts.test.ts`).

**Done when** arrow keys move between results, Enter opens the source of
the focused one, focus is clearly visible in light and dark mode, and the
new key handling has tests.

### Unicode-aware file-name matching (Rust backend)

**Why.** File-name search ignores case for ASCII letters only, because
SQLite's built-in `lower()` doesn't know about other alphabets. So "École"
won't match a search for "école".

**Where.** The name filter in `src-tauri/src/database/files.rs` (the
`nameContains` query) and the pre-filter in `src-tauri/src/search/metadata.rs`.
Two possible approaches: register a Unicode-aware lowercase function with
rusqlite, or store a lowercased file name in a new column through a new
migration in `src-tauri/src/database/migrations/`.

**Done when** names in other alphabets match regardless of case, there are
tests for a few non-ASCII cases, and the limitation is removed from the
README.

### Say when a PDF has pages without text (Rust backend)

**Why.** A PDF with no text at all is reported as probably scanned. But if
only some pages are scans, those pages are skipped silently, and the user
can't tell why a search misses them.

**Where.** `src-tauri/src/extract/pdf.rs` and the error messages in
`src-tauri/src/extract/mod.rs`. The file list shows status messages through
`src/components/library/FileStatusLabel.tsx`.

**Done when** a partly scanned PDF is still indexed, the user can see that
some pages had no text, and there's a test with a generated sample.

### Build Linux packages in CI (packaging)

**Why.** Only the Windows installer is built automatically. Linux users
have to build from source.

**Where.** `.github/workflows/build.yml`. The checks job already installs
everything Tauri needs on Ubuntu, and `npm run build:app` produces `.deb`
and AppImage packages on Linux.

**Done when** CI uploads the Linux packages as artifacts, and someone has
installed one and used it.

### Grow the meaning-search calibration set (search quality)

**Why.** The thresholds that decide what counts as "similar in meaning" were
picked from 24 short synthetic documents. More varied examples would show
whether they hold up.

**Where.** `src-tauri/src/search/calibration.rs`. It needs the real model,
so run it in CI or follow [DEVELOPMENT.md](DEVELOPMENT.md#tests-that-need-the-real-models).
All examples must be invented, with no real people or businesses.

**Done when** the set covers longer, multi-paragraph documents and more
near-misses. If the thresholds in `src-tauri/src/search/semantic.rs` need
to change, the new numbers are recorded in [MODELS.md](MODELS.md).

## Advanced

These need a design discussion before any code. Open an issue or a
discussion first.

- **OCR for scanned PDFs** (Rust backend): render PDF pages to images and
  run them through the existing OCR. Start from `src-tauri/src/extract/` and
  `src-tauri/src/ocr/`; the hard part is rendering without a heavy native
  dependency.
- **Scan only what changed in large folders** (Rust backend): the watcher
  in `src-tauri/src/watching.rs` knows which paths changed, but starts a
  full scan of the folder (`rescan_changed`). The pipeline in
  `src-tauri/src/indexing/pipeline.rs` would need to accept a list of paths.
- **A multilingual embedding model** (search quality): add another
  `Embedder` in `src-tauri/src/embeddings/` and verify it the way
  [MODELS.md](MODELS.md) describes.
