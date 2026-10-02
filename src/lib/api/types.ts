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

/** A folder the user has added for Recall to index. */
export interface Location {
  id: string;
  /** Folder name for display, e.g. "Documents". */
  name: string;
  /** Full absolute path on this computer. */
  path: string;
}

export interface AddLocationResult {
  location: Location;
  /** Previously added folders inside the new one, merged into it. */
  replaced: Location[];
}

/** The operations the frontend can ask the backend to perform. */
export interface RecallApi {
  getAppInfo(): Promise<AppInfo>;
  /**
   * Opens the system folder picker and adds the chosen folder.
   * Resolves to `null` if the user cancels the picker.
   */
  addLocation(): Promise<AddLocationResult | null>;
  listLocations(): Promise<Location[]>;
  /** Removes a folder from the library. Never deletes or changes the files. */
  removeLocation(id: string): Promise<void>;
}
