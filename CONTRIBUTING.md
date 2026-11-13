# Contributing to Recall

Thanks for wanting to help. Recall searches the files on your computer by
meaning, and everything runs locally: no cloud, no accounts, no API keys.
Contributions of every size are welcome, from a fixed typo to a new file
type.

This page covers setup, how the code is laid out, and what makes a pull
request easy to review. If you'd like the big picture first,
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) takes about ten minutes to read.

## The one rule that never bends

Recall must never send anything over the network. That means no telemetry,
no analytics, no cloud AI, no API keys, no update checks and no outbound
requests from the app, ever. The only part of the project that touches the
internet is the model download script you run during setup.

`npm run audit:privacy` checks for this automatically and runs in CI on
every pull request. If your change needs network access to work, it isn't
the right change for Recall. Open a discussion instead and we'll look for a
local way to do it.

## Setting up

You need:

- [Node.js](https://nodejs.org/) 20.19 or newer (or 22.12+)
- [Rust](https://rustup.rs/) 1.94 or newer
- Tauri's system prerequisites for your OS:
  <https://v2.tauri.app/start/prerequisites/>

Then:

```bash
npm install
npm run download-models   # about 145 MB, checksums verified
npm run tauri dev         # opens the app; the first build takes a few minutes
```

Don't skip `download-models`, even if you only want to run the Rust tests.
The model files are bundled into the app, and Tauri's build refuses to
compile the Rust code until they're in `models/`. If you're stuck without
internet access to the model hosts,
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) explains a workaround.

Working only on the interface? You don't need Rust or the models at all.
Run `npm run dev:mock` and open <http://localhost:1420>. See
[docs/FRONTEND_QUICKSTART.md](docs/FRONTEND_QUICKSTART.md).

## How the code is laid out

```text
src/                    React + TypeScript frontend
  pages/                one component per screen
  components/           UI pieces, grouped by feature
  hooks/                data loading (TanStack Query)
  lib/api/              the API contract, the Tauri client and the mock
src-tauri/src/          Rust backend
  commands/             Tauri commands: thin adapters for the frontend
  locations/            which folders can be added
  files/                finding and fingerprinting files
  extract/              text from PDF, DOCX, TXT and MD
  ocr/                  text from images (local ocrs models)
  indexing/             passages, embeddings and the indexing pipeline
  embeddings/           the Embedder interface and the BGE model
  search/               meaning, keyword and file-name search, and ranking
  database/             SQLite storage and migrations
  scanning.rs           runs indexing in the background
  watching.rs           re-indexes folders when files change
docs/                   architecture, API, models, development guides
scripts/                model download, privacy audit, test-data generator
test-data/              synthetic sample files (never real documents)
```

The frontend and backend only meet in one place: the API described in
[docs/API.md](docs/API.md), defined in `src/lib/api/types.ts` and mirrored
by the Rust types in `src-tauri/src/commands/`. If you change the API,
update both sides, the mock (`src/lib/api/mock.ts`) and `docs/API.md` in the
same pull request.

## Checks to run before opening a pull request

```bash
npm test                     # frontend unit tests
npm run build                # type-checks and builds the frontend
npm run audit:privacy        # no network code, no cloud AI
cd src-tauri
cargo fmt                    # formatting
cargo clippy --all-targets -- -D warnings   # lints; CI fails on any warning
cargo test                   # backend unit tests
```

These are the same checks CI runs. There's also a set of tests that need
the real model files (`cargo test -- --ignored`). CI runs them on every pull
request with networking switched off, so you don't have to. If you change
OCR, embeddings or search ranking and want to run them yourself,
[docs/MODELS.md](docs/MODELS.md) explains the extra setup one of them
needs.

## Branches and commits

Fork the repository, then branch from the default branch with a short
descriptive name, like `fix/utf16-bom` or `docs/macos-setup`.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/):
a type, a short summary, and a body that explains why when it isn't
obvious.

```text
feat: read text from scanned PDF pages
fix: refresh search results after a rescan
docs: explain the model download on macOS
```

Commits like `changes` or `wip` make history hard to read, so please
squash or reword them before asking for review. Never commit secrets, real
personal documents, model files or build output.

## Opening a good pull request

Keep each pull request to one change. A focused pull request gets reviewed
faster and is easier to undo if something goes wrong. In the description,
say what you changed, why, and how you tested it: the commands you ran and,
for UI changes, what you clicked and saw. Screenshots help, as long as they
don't show anyone's real files.

Reviewers look for:

- **Privacy first.** No new network access of any kind, and no logging of
  file contents.
- **Honesty in the product.** Recall never shows invented data: no fake
  results, snippets, counts or AI claims. If it can't stand behind
  something, it says so plainly.
- **Plain error messages.** Anything the user sees is written for a person,
  not a developer. Technical detail goes to the log.
- **Tests** for logic you add or change. Backend logic lives in plain Rust
  modules with unit tests next to it; storage functions take a
  `&Connection` so tests can use an in-memory database.
- **Small, clear code.** Comments explain why, not what. Avoid abstractions
  until there's a real second use.
- **Updated docs** when behaviour changes, including the README's
  limitations if you fix one.

## AI models

Models must run locally and have at most 500 million parameters, counted
from the model files, not taken from a web page. If you add or change a
model, record its source, version, parameter count, licence and SHA-256 in
[docs/MODELS.md](docs/MODELS.md), add it to `scripts/download-models.mjs`,
and test with the real model. The fake embedder in
`src-tauri/src/embeddings/fake.rs` only exists to test the plumbing.

## Picking something to work on

[docs/GOOD_FIRST_ISSUES.md](docs/GOOD_FIRST_ISSUES.md) lists self-contained
tasks with pointers to where the code lives. Issues labelled
`good first issue` and `help wanted` are open for anyone. For bigger work,
check [ROADMAP.md](ROADMAP.md), then comment on the issue or open a
discussion before you start, so two people don't build the same thing and
we can agree on an approach.

## Your first contribution, step by step

Say you want to fix a confusing message in the app.

1. Find the text. Most user-facing text is in `src/pages/` and
   `src/components/`; errors from the backend are in `src-tauri/src/`.
   Searching the repository for a few words of the message is the fastest
   way.
2. Fork the repository on GitHub, clone your fork, and create a branch:
   `git checkout -b fix/clearer-empty-search-message`.
3. Make the change. For frontend-only changes, `npm run dev:mock` shows it
   right away in your browser.
4. Run the checks above that apply. For a frontend change that's
   `npm test`, `npm run build` and `npm run audit:privacy`.
5. Commit with a clear message, like
   `fix: explain what to try when a search finds nothing`.
6. Push the branch to your fork and open a pull request. Fill in the
   template, including how you tested it.
7. A reviewer will reply, sometimes with requests for changes. Push more
   commits to the same branch to update the pull request.

## AI-assisted contributions

Using AI tools to help write code or docs is fine. The person opening the
pull request is still responsible for it: you should understand every line,
have run the checks and tested the change yourself, and be able to answer
review questions about it.

Please say in the pull request description when AI helped, and credit it in
the commit with a `Co-Authored-By:` line, for example:

```text
Co-Authored-By: Claude <noreply@anthropic.com>
```

AI tools also make it easy to slip in things Recall doesn't allow, like an
HTTP client or a cloud API call. Double-check that yours didn't.
[AGENTS.md](AGENTS.md) has instructions written for AI coding assistants
working on this repository.

## Community

Everyone taking part is expected to follow the
[Code of Conduct](CODE_OF_CONDUCT.md). Questions go to
[Discussions](https://github.com/akanjiolayinka/RECALL/discussions) (see
[SUPPORT.md](SUPPORT.md)), and security problems should be reported
privately as described in [SECURITY.md](SECURITY.md). How decisions get
made is in [GOVERNANCE.md](GOVERNANCE.md).
