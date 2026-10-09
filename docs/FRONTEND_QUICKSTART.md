# Frontend quickstart

For a frontend developer joining Recall. You can do all UI work **without
Rust, AI models or an index**, using mock mode.

## What Recall is

A desktop app that searches the files on your computer by what you remember
about them. A React frontend talks to a Rust backend; all AI runs locally.
Five-minute overview: [ARCHITECTURE.md](ARCHITECTURE.md).

## What the frontend owns — and doesn't

**Yours:** screens, components, layout, styling, copy, interaction,
accessibility, frontend state, and how API data is displayed.

**Not yours** (and you never need to touch it): the database, file
scanning, text extraction, OCR, embeddings, search ranking. The frontend
only sees the typed API in `src/lib/api/`. If a screen needs data the API
doesn't provide, that's an API change: agree it with a backend contributor
and update [API.md](API.md).

## Setup

You need [Node.js](https://nodejs.org/) 20+.

```bash
npm install
npm run dev:mock
```

Open <http://localhost:1420> in a browser. That's it: no Rust needed.

### Mock mode

`npm run dev:mock` runs the UI against `src/lib/api/mock.ts`: fake folders,
files, scans with progress, search results and evidence. An amber banner
reads "Mock mode — showing fake development data". Click **Library → Add
folder** to get mock files, then search for "budget" or "lease".

Mock mode exists only in development. It turns on only when Vite runs in
dev mode **and** `VITE_RECALL_MOCK=true` (set in `.env.mock`); production
builds strip it out entirely.

To run the real desktop app instead (needs Rust and the Tauri prerequisites,
see the README):

```bash
npm run tauri dev
```

## Where things are

```text
src/
  App.tsx                    app shell: sidebar + current page, Ctrl/⌘+K shortcut
  main.tsx                   React root, TanStack Query provider
  index.css                  Tailwind + design tokens (colors, radius, dark mode)
  pages/                     one file per screen
    SearchPage.tsx           ← the search page
    LibraryPage.tsx
    IndexingPage.tsx
    PrivacyPage.tsx
  components/
    ui/                      shared building blocks: button, input, dialog,
                             progress, empty-state, error-message
    layout/                  Sidebar, PageHeader, BackendStatus
    search/SearchResultCard.tsx   ← a search result
    evidence/                EvidenceViewer (source of a result), DocumentViewer
    library/                 LocationList, FileBrowser, FileKindIcon, FileStatusLabel
    indexing/ScanProgress.tsx
    privacy/PrivacyFact.tsx
  hooks/                     data for components (TanStack Query): useSearch,
                             useLocations, useFiles, useAiStatus, useScanEvents…
  lib/
    api/types.ts             ← every type the backend sends or receives
    api/client.ts            ← `api`: the only way to call the backend
    api/tauri.ts, mock.ts    the real and the mock implementation
    format.ts                numbers, sizes, dates, plurals
    highlight.ts             highlighting matched words
    shortcuts.ts             keyboard shortcuts
    utils.ts                 cn() for merging Tailwind classes
```

**State:** data from the backend lives in TanStack Query (via the hooks).
UI-only state (what's typed, which dialog is open) is plain `useState` in
the component that needs it. There is no global store; add one only if two
distant components truly need the same UI state.

**How a component gets data:** component → hook (`src/hooks/`) → `api`
(`src/lib/api/client.ts`) → backend. Components never call `invoke` directly.

## Common tasks

### Change how a search result looks

Edit `src/components/search/SearchResultCard.tsx`. Its data is the
`SearchResult` type in `src/lib/api/types.ts`. Nothing outside the frontend
is affected.

### Add a component

1. Shared and generic (a badge, a tooltip)? Put it in `components/ui/`.
   shadcn/ui components can be added with `npx shadcn@latest add <name>`.
2. Specific to one feature? Put it in that feature's folder, e.g.
   `components/library/`.
3. Use the design tokens (`bg-card`, `text-muted-foreground`, `border`,
   `bg-primary`…) rather than raw colors, so light and dark mode both work.
4. Use `lucide-react` for icons, with `aria-hidden` when decorative.

### Add a screen

1. Create `src/pages/MyPage.tsx` (start from `IndexingPage.tsx`: a
   `PageHeader` plus content).
2. In `src/components/layout/Sidebar.tsx`, add an id to `PageId` and an entry
   to `NAV_ITEMS` (label + lucide icon).
3. In `src/App.tsx`, add the page to `PAGES`.

### Show new backend data

1. Check [API.md](API.md): is it already available?
2. If not, the API grows: the type goes in `types.ts`, the call in `tauri.ts`
   and `mock.ts`, and a Rust command on the backend. Coordinate first.
3. Add a hook in `src/hooks/` that wraps the call in `useQuery` or
   `useMutation`, and use the hook from your component.

### Handle loading, empty and error states

Every screen that loads data shows all three. Use `EmptyState` and
`ErrorMessage` from `components/ui/`, and show errors with
`errorMessage(error)`; backend messages are already written for users.

## Testing your changes

```bash
npm test          # unit tests (Vitest): src/**/*.test.ts
npm run build     # type-checks everything (tsc) and builds
```

Put logic worth testing (formatting, parsing, highlighting) in `src/lib/` as
plain functions with a `*.test.ts` file next to them. Then check the screen
by hand in mock mode, in both light and dark mode (follow your OS setting),
and with the keyboard only.

## Building for production

The installable app is built from the repository root (needs Rust and the
model files):

```bash
npm run build:app
```

For frontend-only output (`dist/`), `npm run build` is enough.
