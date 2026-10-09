# Recall API

The contract between the React frontend and the Rust backend. The frontend
only ever talks to the backend through this API; it never sees database
tables, model internals or Rust types.

| Side | Where the contract lives |
| --- | --- |
| Frontend (TypeScript) | `src/lib/api/types.ts` — the `RecallApi` interface and every type below |
| Frontend calls | `src/lib/api/tauri.ts` (real backend), `src/lib/api/mock.ts` (development mock) |
| Backend (Rust) | `src-tauri/src/commands/` — one `#[tauri::command]` per operation, registered in `src-tauri/src/lib.rs` |

**Rule:** a change to a command's arguments or result must update both
`types.ts` and the Rust DTO (they mirror each other, field for field), plus
`mock.ts` and this file. Field names are `camelCase` in JSON (Rust uses
`#[serde(rename_all = "camelCase")]`).

## How to call it

UI code calls the `api` object, usually through a hook in `src/hooks/`:

```ts
import { api } from "@/lib/api/client";

const results = await api.search({ query: "project budget" });
```

`client.ts` chooses the real backend, or the mock when running
`npm run dev:mock`. Under the hood, each method is a Tauri `invoke` of the
command named in the table below.

## Conventions

- **Ids are opaque strings** (`"12"`). Don't parse them or build them.
- **Times are numbers**: milliseconds since 1970-01-01 UTC, or `null` when unknown.
- **Sizes are bytes.**
- **Messages are user-facing.** Every `message` and `…Reason` field is plain
  language and can be shown as-is. Technical detail goes to the developer log.
- **Nothing is invented.** Where the backend can't stand behind an answer it
  returns `null` (e.g. `getEvidence`) and the UI must say so rather than guess.

## Errors

Every command either resolves with its result or rejects with an `ApiError`:

```ts
interface ApiError {
  code: string;    // stable, machine-readable
  message: string; // plain language, safe to show
}
```

Show messages with `errorMessage(error)` from `@/lib/api/client`.

| `code` | When |
| --- | --- |
| `folder_not_found` | The chosen folder doesn't exist |
| `not_a_folder` | A file was chosen instead of a folder |
| `already_added` | The folder is already in the library |
| `already_covered` | The folder is inside a folder already in the library |
| `unsupported_location` | The picker returned something that isn't a local folder |
| `location_not_found` | The folder id is unknown (e.g. it was just removed) |
| `file_not_found` | The file id is unknown |
| `file_missing` | The file was moved or deleted since Recall last checked |
| `open_failed` | No app on this computer could open the file |
| `invalid_request` | A malformed request, e.g. an unknown file type filter |
| `database_error` | The index couldn't be read or written |
| `backend_unavailable` | *(frontend only)* The UI is running outside the desktop app and not in mock mode |
| `unknown` | *(frontend only)* An error without the `ApiError` shape |

## Commands

| `api.` method | Tauri command | Arguments | Result |
| --- | --- | --- | --- |
| `getAppInfo()` | `get_app_info` | — | `AppInfo` |
| `addLocation()` | `add_location` | — (opens the system folder picker) | `AddLocationResult \| null` (`null` if cancelled) |
| `listLocations()` | `list_locations` | — | `Location[]` |
| `removeLocation(id)` | `remove_location` | `{ id }` | — |
| `rescanLocation(id)` | `rescan_location` | `{ id }` | — |
| `listFiles(query?)` | `list_files` | `{ query: FileListQuery \| null }` | `FileListPage` |
| `getDocument(fileId)` | `get_document` | `{ fileId }` | `DocumentText \| null` |
| `search(request)` | `search` | `{ request: SearchRequest }` | `SearchResult[]` |
| `getEvidence(passageId)` | `get_evidence` | `{ passageId }` | `Evidence \| null` |
| `openFile(fileId)` | `open_file` | `{ fileId }` | — |
| `getAiStatus()` | `get_ai_status` | — | `AiStatus` |
| `getPrivacyReport()` | `get_privacy_report` | — | `PrivacyReport` |

### Library

```ts
interface Location {
  id: string;
  name: string;            // folder name, e.g. "Documents"
  path: string;            // full path on this computer
  fileCount: number;       // files from this folder in the index
  readCount: number;       // of those, files whose contents were read
  failedCount: number;     // of those, files that couldn't be read
  scan: ScanStatus | null; // latest scan since the app started
}

interface AddLocationResult {
  location: Location;
  replaced: Location[]; // folders already added that were inside the new one, merged into it
}
```

`addLocation` opens the picker **in the backend**, so the backend only ever
receives folders the user really chose. Adding, removing and rescanning
return immediately; scanning runs in the background (see the event below).
Removing a folder deletes it from the index; the user's files are never
touched.

### Files

```ts
type FileKind = "pdf" | "text" | "markdown" | "docx" | "image";
type FileStatus = "pending" | "indexed" | "error";

interface FileListQuery {   // every field optional
  locationId?: string;
  kind?: FileKind;
  nameContains?: string;    // case-insensitive (ASCII letters)
  limit?: number;           // default 100, max 500
  offset?: number;
}

interface FileListPage {
  files: IndexedFile[];     // most recently modified first
  total: number;            // matches across all pages
}

interface IndexedFile {
  id: string; locationId: string; name: string; path: string;
  kind: FileKind; sizeBytes: number;
  modifiedAt: number | null; createdAt: number | null;
  status: FileStatus;       // "pending" = not read yet (images wait for OCR)
  error: string | null;     // why it couldn't be read
}

interface DocumentText {    // getDocument: exactly what Recall extracted
  fileId: string;
  title: string | null; author: string | null;
  pageCount: number | null; // PDFs only
  wordCount: number; chunkCount: number;
  pages: { number: number | null; text: string }[];
}
```

### Search and evidence

```ts
interface SearchRequest { query: string; limit?: number } // limit: default 20, max 50

interface SearchResult {
  id: string;               // unique per result
  passageId: string | null; // for getEvidence; null when only the file name matched
  fileId: string; fileName: string; filePath: string; fileKind: FileKind;
  page: number | null;
  snippet: { text: string; highlight: boolean }[]; // empty for name-only matches
  relevance: number;        // 0–1, for ordering and coarse labels; don't show the number
  matchReasons: string[];   // e.g. ["Mentions “budget”", "File name or title matches “garden”"]
}

interface Evidence {        // before + passage + after = the page's text
  fileId: string; fileName: string; filePath: string; fileKind: FileKind;
  page: number | null;
  before: string; passage: string; after: string;
}
```

Results are one per file, best first. How `relevance` is computed is a
backend detail (see [ARCHITECTURE.md](ARCHITECTURE.md#search)); the UI only
compares it with fixed thresholds for its labels. Evidence is split into
three strings so the UI never has to convert character offsets.

Example:

```ts
const [top] = await api.search({ query: "what was the project budget" });
// top.fileName === "Project Proposal - Riverside Community Garden.pdf"
// top.page === 3
// top.matchReasons === ["Mentions “project”, “budget”", "File name or title matches “project”"]
const evidence = top.passageId ? await api.getEvidence(top.passageId) : null;
// evidence?.passage starts with "Budget\n\nThe estimated project budget is NGN 2,500,000"
```

### Status and privacy

```ts
interface AiStatus {
  semanticSearch: AiFeatureStatus; // local embedding model
  ocr: AiFeatureStatus;            // local OCR model
}
interface AiFeatureStatus {
  available: boolean;
  model: string | null;             // when available
  unavailableReason: string | null; // when not, plain language
}

interface PrivacyReport {           // measured from the running app
  databasePath: string; databaseBytes: number;
  folders: number; files: number; documents: number; passages: number; embeddings: number;
}

interface AppInfo { name: string; version: string; backend: "tauri" | "mock" }
```

## Events

| Event | Payload | Subscribe with |
| --- | --- | --- |
| `scan-progress` | `ScanStatus` | `api.onScanProgress(handler)` (returns an unsubscribe function) |

Sent when a scan starts, at each step change, and at most every 100 ms in
between. `src/hooks/useScanEvents.ts` applies them to the cached folder list
and refreshes folders, files and searches when a scan ends. Counts in a
`ScanStatus` describe that one scan (e.g. a rescan of an unchanged folder
reads 0 files); for what the index holds, use the `Location` counts.

```ts
type ScanState = "discovering" | "hashing" | "reading" | "embedding" | "done" | "failed";

interface ScanStatus {
  locationId: string;
  state: ScanState;
  filesFound: number;       // supported files found
  filesProcessed: number;   // checked so far (including unchanged)
  filesChanged: number;     // new or changed since the last scan
  filesFailed: number;      // couldn't be read from disk
  unreadable: number;       // skipped: no permission
  filesToRead: number;      // documents whose text is read in this scan
  filesRead: number;
  readFailed: number;       // text couldn't be extracted
  passagesToEmbed: number;  // 0 without an embedding model
  passagesEmbedded: number;
  currentFile: string | null; // file name only
  error: string | null;     // plain language, when state is "failed"
}
```
