/**
 * DEVELOPMENT-ONLY mock backend.
 *
 * Lets frontend contributors work on the UI without Rust, AI models or an
 * index. It is only used when `npm run dev:mock` is running (see client.ts);
 * production builds never select it. Everything here is clearly fake data.
 */
import type { AddLocationResult, ApiError, AppInfo, Location, RecallApi } from "./types";

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** Fake folders handed out, in order, each time "Add folder" is clicked. */
const MOCK_FOLDERS = ["/Users/demo/Documents", "/Users/demo/Desktop/Screenshots", "/Users/demo/Notes"];

let mockLocations: Location[] = [];
let nextId = 1;

export const mockApi: RecallApi = {
  async getAppInfo(): Promise<AppInfo> {
    await delay(150);
    return { name: "Recall", version: "0.1.0-mock", backend: "mock" };
  },

  async addLocation(): Promise<AddLocationResult | null> {
    await delay(300);
    const path = MOCK_FOLDERS.find((folder) => !mockLocations.some((l) => l.path === folder));
    if (!path) {
      const error: ApiError = {
        code: "already_added",
        message: "All mock folders are already in your library (mock mode).",
      };
      throw error;
    }
    const location: Location = { id: `loc-${nextId++}`, name: path.split("/").pop()!, path };
    mockLocations = [...mockLocations, location];
    return { location, replaced: [] };
  },

  async listLocations(): Promise<Location[]> {
    await delay(100);
    return mockLocations;
  },

  async removeLocation(id: string): Promise<void> {
    await delay(100);
    mockLocations = mockLocations.filter((location) => location.id !== id);
  },
};
