# Instructions for AI coding assistants

This file is for AI tools working on Recall (Claude Code, Copilot, Cursor
and others). Human contributors will find the same information, at more
length, in [CONTRIBUTING.md](CONTRIBUTING.md).

## What Recall is

A Tauri 2 desktop app that searches the user's own files by meaning. The
backend is Rust (`src-tauri/`), the frontend is React + TypeScript (`src/`).
Indexing, OCR, embeddings, search and the SQLite database all run locally.
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) explains how the pieces fit.

## Hard rules

These are not style preferences. A change that breaks one will be rejected.

1. **No network access from the app.** Don't add HTTP clients, WebSocket
   code, sockets, update checks, telemetry, analytics, crash reporting or
   calls to any cloud AI service, in Rust or in the frontend. Don't add npm
   packages or crates that do any of this. The only code allowed to use the
   internet is `scripts/download-models.mjs`, which the user runs during
   setup.
2. **No API keys, accounts or secrets**, and no feature that needs them.
3. **Run `npm run audit:privacy`** after any change to dependencies, Rust
   code, frontend code or `src-tauri/tauri.conf.json`. It must pass. Don't
   weaken or work around the audit script to make it pass.
4. **Never invent data in the product.** No placeholder results, fake
   snippets, made-up counts or claims about AI that isn't running. If the
   app can't back something up, it says so in plain language.
5. **Models run locally and have at most 500 million parameters**, counted
   from the files. Changes to models must update
   [docs/MODELS.md](docs/MODELS.md) and `scripts/download-models.mjs`.
6. **Never log file contents.** Logs (`eprintln!` with a `recall:` prefix)
   may include file IDs and folder paths, never document text or queries.
7. **Never edit a released database migration.** Schema changes go in a new
   numbered file in `src-tauri/src/database/migrations/`.
8. **Test data is synthetic.** Never add real documents or real people's or
   businesses' names to `test-data/` or test fixtures.

## Where things are

```text
src/pages/              one component per screen
src/components/         UI, grouped by feature; shared pieces in ui/
src/hooks/              TanStack Query hooks: the only way components load data
src/lib/api/            types.ts (the API contract), tauri.ts, mock.ts, client.ts
src-tauri/src/commands/ Tauri commands: thin adapters, no business logic
src-tauri/src/          core modules: files, extract, ocr, indexing, embeddings,
                        search, database, scanning (background), watching
scripts/                download-models.mjs, audit-privacy.mjs, test-data generator
```

The frontend only talks to the backend through `src/lib/api/`. A change to
a command's arguments or result must update `types.ts`, the Rust DTO and
command, `tauri.ts`, `mock.ts` and [docs/API.md](docs/API.md) together.

## Running and testing

```bash
npm install
npm run download-models   # needed before any Rust build; Tauri bundles the models
npm run tauri dev         # full app
npm run dev:mock          # frontend only, fake data, http://localhost:1420
```

Before finishing a change, run what applies:

```bash
npm test
npm run build             # also type-checks
npm run audit:privacy
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Tests marked `#[ignore]` need the real model files and run in CI. If you
can't download the models, [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)
explains how to compile with empty stand-in files; say in your summary
which tests you couldn't run.

## Code style

- Rust: `cargo fmt`, zero clippy warnings, `thiserror` for error types, no
  `unwrap()` outside tests unless a comment explains why it can't fail.
  Core logic lives in plain modules with unit tests next to it; storage
  functions take a `&Connection` so tests can use an in-memory database.
- TypeScript: strict mode, `@/` imports, design tokens (`bg-card`,
  `text-muted-foreground`) rather than raw colours, and loading, empty and
  error states on every screen that loads data.
- Keep changes small and focused. Don't add abstractions, dependencies or
  configuration until there's a real need. Prefer pure-Rust crates.
- Comments explain why, not what.

## Writing style

User-facing text, docs and comments should read like a friendly,
experienced developer wrote them. Use plain, direct language and short
sentences. Explain things as you would to a smart friend new to the
project. Avoid hype and filler words such as "seamless", "robust",
"leverage", "cutting-edge", "delve" and "empower", and generic closing lines.
Use lists only when they're genuinely clearer than a sentence. Error
messages are for users: say what happened and what they can do, and keep
technical detail for the log.

Docs must match the code. Never describe a feature, command or number you
haven't checked. If something is unfinished, say so.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/)
(`feat:`, `fix:`, `docs:`, `test:`, `chore:`, `ci:`) with a short summary
and a body that explains why. Make small, logical commits. Credit your help
honestly with a trailer at the end of each commit message you contributed
to, for example:

```text
Co-Authored-By: Claude <noreply@anthropic.com>
```

The human who opens the pull request is responsible for the change, so
leave them a clear summary of what you did, what you tested and what you
couldn't check.
