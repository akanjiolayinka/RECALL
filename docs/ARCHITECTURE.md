# Recall architecture

Recall is a desktop app (Tauri) with a React frontend and a Rust backend.
Everything — indexing, search, AI models, the database — runs on the user's
computer. This page explains how the pieces fit together.

## The layers

```text
┌──────────────────────────────────────────────┐
│ FRONTEND  src/                               │
│ React + TypeScript: pages, components, hooks │
└──────────────────────┬───────────────────────┘
                       │ src/lib/api  (the only door)
┌──────────────────────▼───────────────────────┐
│ API LAYER  src-tauri/src/commands/           │
│ Tauri commands + events, plain DTOs          │
└──────────────────────┬───────────────────────┘
┌──────────────────────▼───────────────────────┐
│ APPLICATION CORE  src-tauri/src/             │
│ locations · files · extract · indexing ·     │
│ search · scanning · watching                 │
└──────────┬──────────────────────┬────────────┘
┌──────────▼──────────┐ ┌─────────▼────────────┐
│ DATABASE            │ │ LOCAL AI             │
│ database/  SQLite,  │ │ ocr/  (ocrs)         │
│ FTS5, vectors       │ │ embeddings/ (pending)│
└─────────────────────┘ └──────────────────────┘
```

**The rule that keeps it maintainable:** each layer only knows the one
below it through a narrow interface.

- The frontend knows **only** `src/lib/api/` ([API.md](API.md)). It never
  sees SQL, Rust types or models.
- Commands are **thin adapters**: parse the request, call the core, convert
  the result to a DTO. No business logic.
- The core modules are **plain Rust without Tauri** (except `scanning.rs`,
  `watching.rs` and `models.rs`, which are deliberately small glue), so they
  are unit-tested without the app.
- Storage functions take a `&Connection`, so tests use an in-memory database.
- AI models sit behind small interfaces (`Embedder`, `Ocr`), so a model can
  be swapped without touching indexing or search.

## How a file becomes searchable

`indexing/pipeline.rs` runs these steps for one folder. `scanning.rs` runs it
on a background thread and turns its progress into `scan-progress` events.

```text
Folder ─► 1. Find      files/scan.rs      supported files only; skips hidden
          │                               files, Office lock files (~$x.docx),
          │                               node_modules, system folders
          ▼
          2. Fingerprint  files/hash.rs   SHA-256 of new or changed files.
          │               Unchanged size + modification time ⇒ skipped.
          │               New fingerprint ⇒ status "pending" (re-read).
          ▼
          3. Forget       files deleted since last time leave the index
          ▼
          4. Read         extract/        text per page (PDF) or whole file
          │               TXT/MD (UTF-8, UTF-16), PDF (pdf-extract),
          │               DOCX (zip + XML), images (ocr/, if installed)
          │               indexing/chunk.rs: ~1,000-char passages that break at
          │               paragraphs/sentences, overlap slightly, never span pages
          ▼
          5. Embed        indexing/embed.rs: vectors for passages without one
                          (only when an embedding model is installed)
```

Every step only touches what changed, so re-running on an unchanged folder
is cheap. That is why Recall can simply re-scan:

- when the app starts (catches changes made while it was closed),
- when the user presses **Rescan**,
- when `watching.rs` sees file changes (OS notifications, grouped until
  2 s of quiet). Reads are ignored, so Recall's own indexing doesn't trigger
  another scan.

Files that can't be read keep a plain-language reason (damaged, password
protected, scanned PDF without text, over 100 MB) instead of failing the scan.
PDF parsing panics are caught so one bad file can't stop the app.

## The database

One SQLite file, `recall.db`, in the OS app-data folder
(`%APPDATA%\ai.recall.desktop\` on Windows). WAL mode lets a scan write while
the UI reads; every caller opens its own short-lived connection.

| Table | Holds |
| --- | --- |
| `locations` | Folders the user added |
| `files` | Every supported file: path, kind, size, dates, SHA-256, status (`pending` / `indexed` / `error`), error reason |
| `documents` | Extracted text of a file, title, author, page and word counts |
| `chunks` | Passages: text, page number, character offsets into the document |
| `chunks_fts` | SQLite FTS5 full-text index over `chunks.text`, kept in sync by triggers |
| `embeddings` | One vector per passage, tagged with the model that made it |

Deleting a folder cascades to its files, documents, passages and vectors.
The schema is created by numbered migrations in
`src-tauri/src/database/migrations/`, applied once each and tracked with
SQLite's `user_version`. **Never edit a released migration; add a new one.**

## Search

`search/mod.rs` combines up to three signals per file, each scored 0–1:

| Signal | Module | How |
| --- | --- | --- |
| Meaning | `search/semantic.rs` | Cosine similarity between the query's embedding and each passage's (needs a model) |
| Keywords | `search/keyword.rs` | SQLite FTS5 with English stemming; share of the query's words in the best passage, BM25 to break ties |
| File name / title | `search/metadata.rs` | Share of the query's words that start a word of the file name or document title |

```text
Query ─┬─► meaning  (if a model is installed)
       ├─► keywords ──► best passage per file + highlighted snippet
       └─► file name / title
              ▼
        weighted average per file:  65% meaning · 25% keywords · 10% name
        (no model: 50% keywords · 50% name)
              ▼
        one result per file, best first, with plain-language reasons
```

The weights are a tunable starting point, not a measured optimum. Queries
are reduced to quoted words before they reach FTS5, so nothing a user types
can break the query. Filler words ("where", "my", "the") are dropped.

**Evidence.** Each result points to a stored passage. `database/evidence.rs`
returns that passage with the rest of its page, and checks that the stored
offsets still match the stored text; if not, it returns nothing and the UI
says "We couldn't find enough evidence for this result". Recall never
generates or paraphrases text it shows as evidence.

## Local AI

| Feature | Status | Where |
| --- | --- | --- |
| OCR (text in images) | Working: `ocrs` 0.13, ~3M parameters, pure Rust | `src-tauri/src/ocr/` |
| Embeddings (meaning search) | Interface, storage, search and ranking built and unit-tested; **model not installed yet** | `src-tauri/src/embeddings/` |
| Local LLM ("Ask Recall") | Not started (optional) | — |

Model files live in `models/` (downloaded and checksum-verified by
`npm run download-models`, bundled into installers) and are found by
`models.rs`. When a model is missing, the feature reports itself
unavailable with a reason and everything else keeps working. Details,
parameter counts and the verification checklist: [MODELS.md](MODELS.md).

## Privacy

- **No network code in the app.** The Windows, macOS and Linux builds
  contain no HTTP, TLS or WebSocket client library.
- **The window is locked down.** A Content Security Policy only allows the
  frontend to talk to the Recall backend (`connect-src ipc: http://ipc.localhost`).
- **No cloud AI.** All inference runs in-process on local model files.
- **Checked, not assumed.** `npm run audit:privacy` verifies all of the above
  and fails if any of it changes.
- Logs contain technical errors and file ids, never document text.
- The only downloads are the one-time model files, done by a separate
  setup script, never by the app.

## Frontend

```text
src/
  pages/          one component per screen (Search, Library, Indexing, Privacy)
  components/     ui/ (buttons, inputs, dialog…), layout/, and one folder per feature
  hooks/          TanStack Query hooks: the only way components load data
  lib/api/        the API contract, the Tauri client and the dev mock
  lib/            formatting, highlighting, shortcuts (unit-tested)
```

Pages compose components; components get data from hooks; hooks call
`api`. Navigation is a simple `useState` in `App.tsx` (four pages don't
need a router). Server state lives in TanStack Query; `useScanEvents` keeps
it fresh from backend events. See [FRONTEND_QUICKSTART.md](FRONTEND_QUICKSTART.md).

## Where to change things

| To change… | Edit | Doesn't touch |
| --- | --- | --- |
| How a result card looks | `src/components/search/SearchResultCard.tsx` | Rust, database |
| Ranking weights | `src-tauri/src/search/mod.rs` (`WEIGHTS`) | UI |
| Supported file types | `files/kind.rs` + `extract/` | UI (add a label in `FileKindIcon.tsx`) |
| Passage size | `indexing/chunk.rs` (`TARGET_CHARS`) | UI |
| The database schema | a new file in `database/migrations/` | UI |
| The embedding model | an `Embedder` implementation + `embeddings::load()` | indexing, search, UI |
