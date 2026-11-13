# Open source setup: progress notes

A private working file for the maintainer. Delete it before merging
`docs/open-source-setup`.

## Milestones

- [x] 0. Learn the project (this summary)
- [x] 1. Safety first: secrets, env files, personal data
- [x] 2. Rewrite README.md
- [x] 3. Community files
- [ ] 4. Contributor docs and the roadmap
- [ ] 5. GitHub templates and setup
- [ ] 6. AGENTS.md / CLAUDE.md
- [ ] 7. Demo GIF
- [ ] 8. Final check

## How Recall works, end to end

Recall is a Tauri 2 desktop app. The window runs a React + TypeScript
frontend; everything else happens in a Rust backend in the same process.
The two only talk through Tauri commands and one event.

**Adding a folder.** The Library page calls `addLocation()`. The backend
opens the system folder picker itself (`commands/locations.rs`), so it only
ever receives folders the user really chose. `locations::plan_add` rejects
duplicates and folders inside ones already added, and merges child folders
when a parent is added. The folder is saved in SQLite and a background scan
starts.

**Indexing** (`indexing/pipeline.rs`, run on a thread by `scanning.rs`):

1. Walk the folder for supported files: PDF, DOCX, TXT, MD, PNG, JPG/JPEG,
   WEBP. Hidden files, Office lock files and folders like `node_modules` are
   skipped.
2. Fingerprint new or changed files with SHA-256. A file whose size and
   modification time haven't changed is skipped without hashing.
3. Remove files that were deleted since last time.
4. Read the text of pending files: plain text (UTF-8, UTF-16 with BOM),
   Markdown (title from the first heading), PDF (`pdf-extract`, one page at a
   time, panics caught), DOCX (zip + XML, title and author from the metadata),
   and images through the local OCR model (`ocrs`). Files over 100 MB, or
   that are damaged, encrypted or have no text, get a plain-language reason
   instead.
5. Split text into ~1,000-character passages (`indexing/chunk.rs`) that never
   cross a page boundary, and store them in SQLite with an FTS5 index.
6. Run every new passage through the local embedding model
   (BAAI/bge-small-en-v1.5 on the `rten` runtime) and store the 384-number
   vector.

Progress goes to the frontend as `scan-progress` events. Scans run at
startup, on Rescan, and when `watching.rs` (the `notify` crate) sees changes
in a library folder, after 2 seconds of quiet. File reads are ignored, so
Recall's own reading doesn't trigger another scan.

**Search** (`search/mod.rs`): each query is checked three ways.

- Meaning: cosine similarity between the query vector and every stored
  passage vector (a brute-force scan; ~56 ms for 50,000 vectors). A passage
  counts if it scores at least 0.55 and is within 0.06 of the best passage,
  thresholds measured with `search/calibration.rs`.
- Keywords: SQLite FTS5 with an English (porter) stemmer. Scored by the share
  of the query's words found, with BM25 breaking ties.
- File name and title: share of query words that start a word in the name.

Scores are combined 65 / 25 / 10 (meaning / keywords / name), or 50 / 50
when no embedding model is loaded. Each result carries a passage, a snippet
and plain-language reasons ("Mentions “budget”", "Similar in meaning").

**Evidence.** "Show source" calls `getEvidence(passageId)`, which returns the
stored passage plus the rest of its page as three strings. It checks that
the stored offsets still match the stored text and returns nothing if they
don't. The UI never generates text.

**Models** are found in `$RECALL_MODELS_DIR`, then the installed app's
resources folder, then `models/` in debug builds (`models.rs`). If a model is
missing, its feature reports itself unavailable with a reason and the rest
keeps working. `scripts/download-models.mjs` fetches the four files (two ocrs
ONNX files from S3, BGE's `model.onnx` and `tokenizer.json` from Hugging
Face at a pinned revision), checks SHA-256 and refuses mismatches. Installers
bundle them, so the installed app never downloads anything.

**Where data lives.** One SQLite file, `recall.db`, in the OS app-data folder
(the identifier is `ai.recall.desktop`). WAL mode, schema in numbered
migrations tracked with `user_version`.

**Frontend.** Four pages (Search, Library, Indexing, Privacy), navigation
by `useState` in `App.tsx`, data through TanStack Query hooks in
`src/hooks/`, and one door to the backend: `src/lib/api/client.ts`, which
picks `tauri.ts` or `mock.ts`. Mock mode only switches on when Vite runs in
dev mode and `VITE_RECALL_MOCK=true` (set by `.env.mock` via
`npm run dev:mock`), so production builds can't ship it.

**Privacy guarantees and how they're enforced.**
`scripts/audit-privacy.mjs` checks four things: no HTTP/TLS/WebSocket crates
in the Rust dependency tree for Windows, macOS (x86 and ARM) and Linux; no
cloud AI service names in the source; no `fetch`/`XMLHttpRequest`/
`WebSocket`/`sendBeacon` in the frontend; and a CSP whose `connect-src` only
allows the Tauri IPC. It already runs in CI on every push and PR.

**CI** (`.github/workflows/build.yml`) on every push and PR:
- frontend tests and build
- model download
- `cargo fmt`, clippy with warnings as errors, and `cargo test`
- the privacy audit
- an independent parameter count with the `onnx` package
- reference vectors from sentence-transformers
- the real-model tests with networking switched off
- a Windows installer uploaded as an artifact

**Tests.** 101 Rust tests: 94 run by default and 7 are ignored because they
need model files. There are 9 frontend tests (Vitest) in 3 files under
`src/lib/`. No component or end-to-end tests yet.

## Real versus planned

Working and backed by code and tests:
- folders
- the file types listed above
- extraction
- OCR for images
- keyword, file-name and meaning search, with hybrid ranking
- evidence
- incremental indexing
- file watching
- the Privacy page
- the privacy audit
- Windows installer builds
- offline operation of the indexing and search core (CI)

Tested by a person:
- the Windows installer before meaning search was added (folders, indexing,
  OCR, search, evidence)
- the Linux `.deb` offline, also before meaning search

Not tested by a person yet:
- meaning search in the installed app
- the narrow-window layout
- Windows offline
- macOS at all

Not built:
- OCR for scanned PDFs (detected and reported only)
- "Ask Recall" local LLM
- non-English meaning search
- Unicode-aware file-name matching
- smarter handling of very large folders
- releases (installers only exist as 14-day CI artifacts that need a GitHub
  login to download)
- macOS and Linux installers in CI

## Docs that are wrong, stale or missing

1. **CONTRIBUTING says `cargo test` needs no models. It does.** Tauri's build
   script refuses to compile unless the four bundled model files exist, so
   `cargo test` and `cargo clippy` fail on a fresh clone until
   `npm run download-models` has run (~145 MB, needs internet). Should be
   stated plainly in setup.
2. **`cargo test -- --ignored` fails for most contributors.**
   `matches_the_reference_implementation` needs `reference.json`, which only
   `scripts/bge_reference.py` creates (Python, sentence-transformers, PyTorch
   and Hugging Face access). CONTRIBUTING suggests running it without
   mentioning this.
3. **README's installer notes are out of date.** "Linux `.deb` (18 MB)"
   predates the embedding model (~130 MB more). The offline/Windows test
   notes are accurate but read like a test log rather than user information.
4. **The narrow-window layout is unreachable in the real app.**
   `tauri.conf.json` sets `minWidth: 900`, but the sidebar only collapses
   below 768 px. The fixes still help the content area, but the icon-only
   sidebar only appears in mock mode in a browser. My earlier test
   instructions ("make the window a third of your screen") were wrong for
   the app.
5. **Stale internal references.** Comments mention "Milestone 5/7/17" from
   the original build plan (`files/mod.rs`, `models.rs`,
   `migrations/0001_initial.sql`). Harmless but meaningless to newcomers. The
   migration file shouldn't be edited (released migration), but the two
   `.rs` comments can be.
6. **CI comment is stale.** The Windows job says it downloads "the OCR
   models"; it downloads all four model files.
7. **README "Download a test build"** points people at CI artifacts. That
   works for collaborators but strangers need a GitHub login, and they
   expire after 14 days. A real release is better (Milestone 5 checklist).
8. **macOS data path** isn't documented; only Windows is.
9. **Mock mode** shows invented files ("Q3 Budget.docx" under
   `/Users/demo/...`), not `test-data/`; search matches file-name words only;
   meaning search and OCR aren't simulated. FRONTEND_QUICKSTART describes
   this correctly, but it matters for the demo GIF (see open questions).

Everything else I checked matched the code: the API contract in
`docs/API.md`, the architecture description, the model records in
`docs/MODELS.md`, the commands in `package.json`, the licence line (MIT,
© 2026 Olayinka Akanji), and the privacy claims (no network crates, strict
CSP, logs contain ids and paths but never document text).

## Open questions for the maintainer

1. **Hackathon line.** README line 1 is "This project was submitted to the
   ryze.ai hackathon by Olayinka Akanji." It was a hackathon rule that it be
   the very first line. Keep it there, move it lower (e.g. to an "About"
   section), or drop it?
2. **Demo GIF.** Mock mode doesn't show real behaviour: a vague description
   like "the invoice about the water pump" would find nothing, because the
   mock only matches file names and there is no such file. Options: (a)
   record the real app myself, which needs the BGE model and a display (this
   environment can't download the model; a GitHub Actions job could, with
   Xvfb and ffmpeg); (b) write `docs/assets/RECORDING.md` so you record it
   on Windows; (c) a mock-mode GIF clearly labelled as UI-only. I'd suggest
   (a) if you're happy with a CI job that produces the GIF, else (b).
3. **Real company names in synthetic text.**
   - `test-data/` uses invented people (Amaka Obi, Tunde Bello, Chidi Eze)
     but "Lekki Homes Ltd" and "Soundwave Electronics, 14 Market Road,
     Ikeja", which could match real businesses.
   - The calibration set in `src-tauri/src/search/calibration.rs` names real
     organisations: Access Bank, Lagos University Teaching Hospital, Kenya
     Airways, Ikeja Electric, Coscharis Motors, Nigeria Immigration Service,
     plus Toyota and Samsung products.

   None is personal data, but you may prefer invented names throughout. I'll
   raise it properly in Milestone 1.
4. **App identifier `ai.recall.desktop`.** It suggests ownership of the
   recall.ai domain. Changing it moves where existing installs keep their
   database, so it's a decision for you, not a docs fix.
5. **Contributor history.** The first 23 commits are authored by "Claude".
   You asked earlier to remove Claude as a contributor; that needs a history
   rewrite and force-push that I'm not able to do. For this project you now
   want Claude credited via Co-Authored-By, which is what I'll do on this
   branch.

## Milestone 1 results

**Secrets.** None in the working tree or in any of the 27 commits of
history (searched for key/token/password assignments, GitHub, OpenAI, HF
and AWS key shapes, private keys, emails and home-directory paths). Author
emails are GitHub and Anthropic no-reply addresses. Paths in tests and mock
data are made up (`/home/me/...`, `/Users/demo/...`). Nothing needs
rotating and no history clean-up is needed.

**`.env.mock`** only held `VITE_RECALL_MOCK=true`. Mock mode now checks
Vite's own mode (`vite --mode mock`), so the file is gone, along with its
`.gitignore` exception. Checked: `npm run dev:mock` shows mock data, plain
`npm run dev` doesn't, and the production bundle contains no mock code.

**`.gitignore`** covers `.env`, `.env.*`, `*.local`, `models/`, `dist/`,
`target/`, SQLite files and Python virtualenvs (checked with
`git check-ignore`).

**Network calls.** The app has none. Outside the app: the model download
script (allowed), `scripts/bge_reference.py` (CI-only dev tool;
sentence-transformers downloads the model from Hugging Face), and CI's own
downloads. The privacy audit already ran in CI on every push and PR; it now
also catches Rust socket code, `EventSource` and hard-coded web addresses
in the frontend, HTTP/updater/analytics npm packages and crates, and
network permissions on the window. Each new check was tried against a
planted violation.

**Sample data.** People's names were invented. Real or possibly real
organisation names (in `test-data/` and the calibration set in
`search/calibration.rs`) are replaced with clearly fictional "Example …"
names. `test-data/` was regenerated with `scripts/generate_test_data.py`;
the Word file is unchanged (only zip timestamps differed, so it was kept).

**To re-check once the branch is pushed:** the calibration set's wording
changed slightly, so its similarity numbers will move a little. CI's
real-model tests will show whether the recorded results (23/25 right,
7 wrong, 23 of 24 first, 1 unrelated match) still hold. If they shift, the
numbers in `docs/MODELS.md`, `docs/ARCHITECTURE.md` and `search/semantic.rs`
need updating to match. The OCR test on the new receipt was run here.

**App identifier** (question 4): changed from `ai.recall.desktop`, which
implies owning recall.ai, to `io.github.akanjiolayinka.recall`. Cheapest
now, before any public release. A test build that's already installed will
start with an empty index after updating; noted in the changelog.
