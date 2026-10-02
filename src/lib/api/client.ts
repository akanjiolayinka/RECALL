/**
 * The single entry point the UI uses to talk to the backend.
 *
 *   import { api } from "@/lib/api/client";
 *   const info = await api.getAppInfo();
 *
 * Mock data is used only when BOTH are true:
 *   - this is a Vite development build (`import.meta.env.DEV`), and
 *   - VITE_RECALL_MOCK is "true" (set by `npm run dev:mock`).
 * In production builds `import.meta.env.DEV` is false, so the mock branch is
 * removed by the bundler and can never be shipped as real data.
 */
import { mockApi } from "./mock";
import { tauriApi } from "./tauri";
import type { RecallApi } from "./types";

export const isMockMode = import.meta.env.DEV && import.meta.env.VITE_RECALL_MOCK === "true";

export const api: RecallApi = isMockMode ? mockApi : tauriApi;

export type * from "./types";
export { errorMessage } from "./errors";
