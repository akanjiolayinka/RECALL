# Development guide

This is the practical companion to [ARCHITECTURE.md](ARCHITECTURE.md): how
to run Recall while you work on it, how to debug it, and how to get past the
setup problems people usually hit. For first-time setup, start with
[CONTRIBUTING.md](../CONTRIBUTING.md).

## The three ways to run Recall

**The full app**: `npm run tauri dev`. This starts Vite for the frontend,
compiles the Rust backend and opens the desktop window. Frontend changes
reload instantly; Rust changes trigger a recompile and restart. The first
build takes a few minutes because every dependency is compiled with
optimisations on, even in dev builds (`[profile.dev.package."*"]` in
`src-tauri/Cargo.toml`). Hashing, SQLite and the AI models are painfully
slow without them. Your own Rust code stays quick to rebuild.

**The interface on its own**: `npm run dev:mock`, then
<http://localhost:1420> in a browser. The real backend is replaced by
`src/lib/api/mock.ts`, which hands out invented folders and files. Its
search only matches words in file names, and meaning search and OCR aren't
simulated. It's for building and styling screens, not for judging search
quality. See [FRONTEND_QUICKSTART.md](FRONTEND_QUICKSTART.md).

**The backend's logic on its own**: `cargo test` in `src-tauri/`. Indexing,
extraction, search and ranking are plain Rust modules that don't need the
app running, and most tests use an in-memory database.

## How the frontend and backend talk

The frontend never calls Rust directly. Every component gets data from a
hook in `src/hooks/`, every hook calls the `api` object from
`src/lib/api/client.ts`, and that object is either the Tauri client
(`tauri.ts`) or the mock (`mock.ts`).

In the Tauri client, each method is an `invoke` of a Rust function marked
`#[tauri::command]` in `src-tauri/src/commands/`. Arguments and results are
plain JSON. The Rust types use `#[serde(rename_all = "camelCase")]`, so
field names match the TypeScript types in `src/lib/api/types.ts` exactly.
Errors come back as `{ code, message }`, where `message` is already written
for the user.

Scan progress goes the other way, as a `scan-progress` event that the
backend emits and `api.onScanProgress()` listens to. `useScanEvents` turns
those events into fresh data for the Library and Indexing pages.

To add a new command:

1. Write the function in a module under `src-tauri/src/commands/`. Keep it
   thin: parse the input, call into the core modules, return a DTO.
2. Register it in `generate_handler!` in `src-tauri/src/lib.rs`.
3. Add its types and method to `src/lib/api/types.ts`, implement it in
   `tauri.ts` and `mock.ts`, and add a hook if components will use it.
4. Document it in [API.md](API.md).

## Debugging

**Rust.** Backend messages are printed with `eprintln!` and show up in the
terminal running `npm run tauri dev`, prefixed with `recall:`. They describe
what failed and include file IDs or folder paths, but never file contents,
and new log lines should keep it that way.

**Frontend.** In dev builds, right-click in the app window and choose
**Inspect** to open the browser developer tools. The console, network tab
and React components all work as usual. The network tab should only ever
show requests to Recall's own backend.

**The database.** Recall keeps its index in `recall.db` inside the app-data
folder: `%APPDATA%\io.github.akanjiolayinka.recall\` on Windows,
`~/Library/Application Support/io.github.akanjiolayinka.recall/` on macOS
and `~/.local/share/io.github.akanjiolayinka.recall/` on Linux. Dev builds
and an installed Recall use the same folder. You can open the file with the
`sqlite3` command-line tool or any SQLite browser while the app is closed.
To start from scratch, quit Recall and delete the folder.

**Models.** Recall looks for model files in `$RECALL_MODELS_DIR` if it's
set, then in the installed app's resources, then in `models/` at the
repository root (dev builds only). The Privacy page shows whether each model
loaded, and if it didn't, why.

## Tests that need the real models

Seven Rust tests are marked `#[ignore]` because they need the model files:
OCR on the sample receipt, BGE's parameter count and similarity checks,
meaning search over `test-data/`, and the calibration set in
`src-tauri/src/search/calibration.rs`. CI runs all of them on every push and
pull request, with networking switched off.

To run them yourself:

```bash
npm run download-models
cd src-tauri
cargo test -- --ignored --nocapture
```

One of them, `matches_the_reference_implementation`, also needs reference
vectors from the official Python implementation. Making them needs Python,
PyTorch and sentence-transformers, so most people let CI handle it.
[MODELS.md](MODELS.md) has the commands. To run the others and skip that
one:

```bash
cargo test -- --ignored --skip matches_the_reference_implementation
```

`--nocapture` prints the similarity tables, which is the point when you're
tuning search.

## Common setup problems

**"resource path `../models/...` doesn't exist" when building.** The model
files haven't been downloaded. Run `npm run download-models` from the
repository root. If you can't reach the model hosts (some networks block
Hugging Face), you can still compile by creating empty stand-in files:

```bash
mkdir -p models/ocrs models/bge-small-en-v1.5
touch models/ocrs/text-detection.onnx models/ocrs/text-recognition.onnx \
      models/bge-small-en-v1.5/model.onnx models/bge-small-en-v1.5/tokenizer.json
```

The app will run, but OCR and meaning search will report that their models
couldn't be loaded, and keyword and file-name search still work. When you
do have access, `npm run download-models` replaces the stand-ins, because
their checksums don't match.

**"package requires rustc 1.94" or similar.** Your Rust toolchain is older
than Recall needs. Run `rustup update stable`.

**Vite complains about the Node.js version.** Recall's Vite needs Node.js
20.19+ or 22.12+.

**"Port 1420 is already in use".** Another Vite server (often a forgotten
`npm run dev:mock`) is still running. Stop it and try again; Tauri expects
the frontend on exactly that port.

### Windows

- Install the Microsoft C++ Build Tools with the "Desktop development with
  C++" workload, and make sure WebView2 is present (it ships with Windows 11
  and recent Windows 10). Tauri's prerequisites page has the details.
- The first build is slow, and antivirus scanning of `src-tauri/target/`
  makes it slower still. Excluding that folder from real-time scanning helps
  if you're allowed to.
- Installers built locally or by CI aren't code-signed, so SmartScreen
  warns about them. Choose **More info**, then **Run anyway**.

### macOS

- Install Xcode's command-line tools with `xcode-select --install`.
- Recall hasn't been tested on macOS yet. If you get it running, or hit a
  problem, please tell us in an issue or a discussion; it would help a lot.

### Linux

- Tauri needs WebKitGTK and a few other system libraries. On Debian or
  Ubuntu, this is what CI installs:

  ```bash
  sudo apt-get install libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
    libayatana-appindicator3-dev librsvg2-dev
  ```

  Other distributions have equivalents; Tauri's prerequisites page lists
  them.
- File watching uses inotify, which limits how many folders one user can
  watch. With very large libraries Recall may log
  `recall: could not watch ...`. Raising `fs.inotify.max_user_watches` with
  `sysctl` fixes it. Rescan still works without the watcher.
