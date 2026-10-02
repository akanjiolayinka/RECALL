This project was submitted to the ryze.ai hackathon by [NAME OF SUBMITTER].

# Recall

**Your private AI memory. Everything searchable. Nothing uploaded.**

Recall is a local-first desktop app for searching the files on your computer by
what you remember about them, not by filename. All AI runs on your device.

> **Status: early development (Milestone 2 of 15).** The desktop app launches
> and you can choose folders. Indexing and search are **not implemented yet**.
> This README only describes what currently works; it will grow as features land.

## What works today

| Feature | Status |
| --- | --- |
| Desktop app launches (Tauri + React) | Working |
| Frontend talks to the Rust backend | Working |
| Development mock mode for UI work | Working |
| Choose folders in Library (system folder picker) | Working — not yet saved between launches |
| File scanning, indexing, search, OCR, embeddings | Not implemented yet |

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
  components/library/ Library screen pieces (folder list)
  hooks/             React hooks that load data through the API client
  lib/api/           The only place the frontend talks to the backend
src-tauri/           Rust backend (Tauri)
  src/commands/      Commands the frontend can call (thin adapters)
  src/locations/     Rules for which folders can be added (unit-tested)
  src/error.rs       The error format every command returns
```
