import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  AddLocationResult,
  ApiError,
  AppInfo,
  DocumentText,
  FileListPage,
  Location,
  RecallApi,
  ScanStatus,
  SearchCapabilities,
  SearchResult,
} from "./types";

/** Backend event names. Must match the constants in src-tauri/src/. */
const SCAN_PROGRESS_EVENT = "scan-progress";

/** Convert whatever a Tauri command rejected with into our ApiError shape. */
function toApiError(err: unknown): ApiError {
  if (err && typeof err === "object" && "code" in err && "message" in err) {
    return err as ApiError;
  }
  return { code: "unknown", message: String(err) };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    const error: ApiError = {
      code: "backend_unavailable",
      message:
        "The Recall backend isn't available. Start the desktop app with `npm run tauri dev`, or use mock mode with `npm run dev:mock`.",
    };
    throw error;
  }
  try {
    return await invoke<T>(command, args);
  } catch (err) {
    throw toApiError(err);
  }
}

/** The real API, backed by Rust commands registered in src-tauri/src/lib.rs. */
export const tauriApi: RecallApi = {
  getAppInfo: () => call<AppInfo>("get_app_info"),
  addLocation: () => call<AddLocationResult | null>("add_location"),
  listLocations: () => call<Location[]>("list_locations"),
  removeLocation: (id) => call<void>("remove_location", { id }),
  rescanLocation: (id) => call<void>("rescan_location", { id }),
  listFiles: (query) => call<FileListPage>("list_files", { query: query ?? null }),
  getDocument: (fileId) => call<DocumentText | null>("get_document", { fileId }),
  search: (request) => call<SearchResult[]>("search", { request }),
  openFile: (fileId) => call<void>("open_file", { fileId }),
  getSearchCapabilities: () => call<SearchCapabilities>("get_search_capabilities"),
  onScanProgress: async (handler) => {
    if (!isTauri()) return () => {};
    return listen<ScanStatus>(SCAN_PROGRESS_EVENT, (event) => handler(event.payload));
  },
};
