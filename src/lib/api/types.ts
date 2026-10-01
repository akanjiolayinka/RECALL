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

/** The operations the frontend can ask the backend to perform. */
export interface RecallApi {
  getAppInfo(): Promise<AppInfo>;
}
