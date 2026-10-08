/**
 * Data shapes shared between the React frontend and the Rust backend.
 *
 * These are the API contract (see docs/API.md). They must match the
 * serialized Rust DTOs in src-tauri/src/commands/. UI code should depend only
 * on these types, never on backend internals.
 */

/** Basic information about the running application. */
export interface AppInfo {
  name: string;
  version: string;
  /** Which implementation answered: the real Rust backend or dev-only mock data. */
  backend: "tauri" | "mock";
}

/** Error shape returned by every backend command. */
export interface ApiError {
  /** Stable machine-readable code, e.g. "backend_unavailable". */
  code: string;
  /** Human-readable message that is safe to show to the user. */
  message: string;
}

/** Where a folder scan is up to. */
export type ScanState = "discovering" | "hashing" | "reading" | "done" | "failed";

/**
 * Progress of scanning one folder. Returned inside `Location` and also pushed
 * live through the "scan-progress" event (see `RecallApi.onScanProgress`).
 */
export interface ScanStatus {
  locationId: string;
  state: ScanState;
  /** Supported files found so far. */
  filesFound: number;
  /** Files checked so far (including unchanged ones and ones that failed). */
  filesProcessed: number;
  /** Files that were found but couldn't be read. */
  filesFailed: number;
  /** Folders/files skipped because Recall wasn't allowed to read them. */
  unreadable: number;
  /** Documents whose text needs reading in this scan. */
  filesToRead: number;
  /** Documents read so far (including ones whose text couldn't be read). */
  filesRead: number;
  /** Documents whose text couldn't be read (damaged, scanned, too large…). */
  readFailed: number;
  /** Name of the file being processed right now. */
  currentFile: string | null;
  /** User-facing reason, when `state` is "failed". */
  error: string | null;
}

/** A folder the user has added for Recall to index. */
export interface Location {
  id: string;
  /** Folder name for display, e.g. "Documents". */
  name: string;
  /** Full absolute path on this computer. */
  path: string;
  /** Files from this folder currently saved in the index. */
  fileCount: number;
  /** Latest scan progress since the app started, or null if not scanned yet. */
  scan: ScanStatus | null;
}

/** Supported file types. */
export type FileKind = "pdf" | "text" | "markdown" | "docx" | "image";

/**
 * Where a file is in Recall's pipeline:
 * - "pending": found, but its contents haven't been read yet (images wait for OCR)
 * - "indexed": contents read and searchable
 * - "error": couldn't be read; see `IndexedFile.error`
 */
export type FileStatus = "pending" | "indexed" | "error";

/** A supported file Recall found in one of the user's folders. */
export interface IndexedFile {
  id: string;
  locationId: string;
  name: string;
  path: string;
  kind: FileKind;
  sizeBytes: number;
  /** Milliseconds since the Unix epoch, or null if unknown. */
  modifiedAt: number | null;
  createdAt: number | null;
  status: FileStatus;
  /** User-facing reason the file couldn't be read, or null if it's fine. */
  error: string | null;
}

/** Filters for `listFiles`. Every field is optional. */
export interface FileListQuery {
  locationId?: string;
  kind?: FileKind;
  /** Case-insensitive match against the file name. */
  nameContains?: string;
  /** Page size (default 100, maximum 500). */
  limit?: number;
  offset?: number;
}

export interface FileListPage {
  /** Most recently modified first. */
  files: IndexedFile[];
  /** Number of files matching the filters, across all pages. */
  total: number;
}

/** One page of a document; `number` is null for formats without pages. */
export interface DocumentPage {
  number: number | null;
  text: string;
}

/** The text Recall extracted from a file. */
export interface DocumentText {
  fileId: string;
  title: string | null;
  author: string | null;
  /** Number of pages, for formats that have pages (PDF). */
  pageCount: number | null;
  wordCount: number;
  /** Number of searchable passages the text was split into. */
  chunkCount: number;
  pages: DocumentPage[];
}

export interface SearchRequest {
  query: string;
  /** Maximum number of results (default 20, maximum 50). */
  limit?: number;
}

/** Part of a result snippet; `highlight` marks words that matched the search. */
export interface SnippetPart {
  text: string;
  highlight: boolean;
}

/** A file that matched a search, with its best-matching passage. */
export interface SearchResult {
  /** Id of the matching passage. */
  id: string;
  fileId: string;
  fileName: string;
  filePath: string;
  fileKind: FileKind;
  /** Page of the passage, for formats with pages. */
  page: number | null;
  /** The matching passage, shortened around the matches. */
  snippet: SnippetPart[];
  /**
   * 0–1 relevance for display; not comparable across searches. Today this is
   * the share of the search's words found in the passage (1 = all of them).
   */
  relevance: number;
  /** Plain-language reasons, e.g. `Mentions “budget”`. Safe to show as-is. */
  matchReasons: string[];
}

/** Call to stop listening to an event. */
export type Unsubscribe = () => void;

export interface AddLocationResult {
  location: Location;
  /** Previously added folders inside the new one, merged into it. */
  replaced: Location[];
}

/** The operations the frontend can ask the backend to perform. */
export interface RecallApi {
  getAppInfo(): Promise<AppInfo>;
  /**
   * Opens the system folder picker, adds the chosen folder and starts scanning it.
   * Resolves to `null` if the user cancels the picker.
   */
  addLocation(): Promise<AddLocationResult | null>;
  listLocations(): Promise<Location[]>;
  /** Removes a folder from the library. Never deletes or changes the files. */
  removeLocation(id: string): Promise<void>;
  /** Checks a folder again for new, changed and deleted files. Progress arrives via `onScanProgress`. */
  rescanLocation(id: string): Promise<void>;
  listFiles(query?: FileListQuery): Promise<FileListPage>;
  /** The text extracted from a file, or null if it hasn't been read. */
  getDocument(fileId: string): Promise<DocumentText | null>;
  /** Search the text of indexed files. Best matches first, one result per file. */
  search(request: SearchRequest): Promise<SearchResult[]>;
  /** Open an indexed file in its default app on this computer. */
  openFile(fileId: string): Promise<void>;
  /** Subscribe to live scan progress. Resolves to a function that unsubscribes. */
  onScanProgress(handler: (status: ScanStatus) => void): Promise<Unsubscribe>;
}
