import { invoke, isTauri } from "@tauri-apps/api/core";

import type { ApiError, AppInfo, RecallApi } from "./types";

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
};
