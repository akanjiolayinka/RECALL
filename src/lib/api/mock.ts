/**
 * DEVELOPMENT-ONLY mock backend.
 *
 * Lets frontend contributors work on the UI without Rust, AI models or an
 * index. It is only used when `npm run dev:mock` is running (see client.ts);
 * production builds never select it. Everything here is clearly fake data.
 */
import type { AppInfo, RecallApi } from "./types";

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const mockApi: RecallApi = {
  async getAppInfo(): Promise<AppInfo> {
    await delay(150);
    return { name: "Recall", version: "0.1.0-mock", backend: "mock" };
  },
};
