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
export type ScanState = "discovering" | "hashing" | "done" | "failed";

/**
 * Progress of scanning one folder. Returned inside `Location` and also pushed
 * live through the "scan-progress" event (see `RecallApi.onScanProgress`).
 */
export interface ScanStatus {
  locationId: string;
  state: ScanState;
  /** Supported files found so far. */
  filesFound: number;
  /** Files read and fingerprinted so far (including ones that failed). */
  filesProcessed: number;
  /** Files that were found but couldn't be read. */
  filesFailed: number;
  /** Folders/files skipped because Recall wasn't allowed to read them. */
  unreadable: number;
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
  /** Latest scan progress, or null if the folder hasn't been scanned yet. */
  scan: ScanStatus | null;
}

/** Supported file types. */
export type FileKind = "pdf" | "text" | "markdown" | "docx" | "image";

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
  /** Scans a folder again from scratch. Progress arrives via `onScanProgress`. */
  rescanLocation(id: string): Promise<void>;
  listFiles(query?: FileListQuery): Promise<FileListPage>;
  /** Subscribe to live scan progress. Resolves to a function that unsubscribes. */
  onScanProgress(handler: (status: ScanStatus) => void): Promise<Unsubscribe>;
}
