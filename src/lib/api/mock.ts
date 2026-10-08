/**
 * DEVELOPMENT-ONLY mock backend.
 *
 * Lets frontend contributors work on the UI without Rust, AI models or an
 * index. It is only used when `npm run dev:mock` is running (see client.ts);
 * production builds never select it. Everything here is clearly fake data.
 */
import type {
  AddLocationResult,
  ApiError,
  AppInfo,
  FileKind,
  FileListPage,
  FileListQuery,
  IndexedFile,
  Location,
  RecallApi,
  ScanStatus,
} from "./types";

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** Fake folders handed out, in order, each time "Add folder" is clicked. */
const MOCK_FOLDERS: { path: string; files: [string, FileKind, number][] }[] = [
  {
    path: "/Users/demo/Documents",
    files: [
      ["Project Proposal.pdf", "pdf", 2_400_000],
      ["Q3 Budget.docx", "docx", 85_000],
      ["Apartment move checklist.md", "markdown", 3_200],
      ["Meeting notes 2024-05-02.txt", "text", 4_100],
      ["Lease agreement.pdf", "pdf", 640_000],
    ],
  },
  {
    path: "/Users/demo/Desktop/Screenshots",
    files: [
      ["Database architecture.png", "image", 410_000],
      ["Headphones receipt.jpg", "image", 220_000],
      ["Error message.png", "image", 98_000],
    ],
  },
  { path: "/Users/demo/Notes", files: [["Ideas.md", "markdown", 1_800]] },
];

let locations: Location[] = [];
let files: IndexedFile[] = [];
let nextId = 1;
const listeners = new Set<(status: ScanStatus) => void>();

function setScan(status: ScanStatus) {
  locations = locations.map((l) => (l.id === status.locationId ? { ...l, scan: status } : l));
  listeners.forEach((listener) => listener(status));
}

/** Pretend to scan a folder, emitting progress like the real backend. */
async function simulateScan(location: Location) {
  const folder = MOCK_FOLDERS.find((f) => f.path === location.path)!;
  const total = folder.files.length;
  const base: ScanStatus = {
    locationId: location.id,
    state: "discovering",
    filesFound: 0,
    filesProcessed: 0,
    filesFailed: 0,
    unreadable: 0,
    currentFile: null,
    error: null,
  };
  setScan(base);
  await delay(400);
  setScan({ ...base, state: "hashing", filesFound: total });
  for (let i = 0; i < total; i++) {
    setScan({ ...base, state: "hashing", filesFound: total, filesProcessed: i, currentFile: folder.files[i][0] });
    await delay(250);
  }
  if (!locations.some((l) => l.id === location.id)) return;
  files = files
    .filter((f) => f.locationId !== location.id)
    .concat(
      folder.files.map(([name, kind, sizeBytes], i) => ({
        id: `file-${location.id}-${i}`,
        locationId: location.id,
        name,
        path: `${location.path}/${name}`,
        kind,
        sizeBytes,
        modifiedAt: Date.now() - i * 86_400_000,
        createdAt: null,
        error: null,
      })),
    );
  setScan({ ...base, state: "done", filesFound: total, filesProcessed: total });
}

export const mockApi: RecallApi = {
  async getAppInfo(): Promise<AppInfo> {
    await delay(150);
    return { name: "Recall", version: "0.1.0-mock", backend: "mock" };
  },

  async addLocation(): Promise<AddLocationResult | null> {
    await delay(300);
    const folder = MOCK_FOLDERS.find((f) => !locations.some((l) => l.path === f.path));
    if (!folder) {
      const error: ApiError = {
        code: "already_added",
        message: "All mock folders are already in your library (mock mode).",
      };
      throw error;
    }
    const location: Location = {
      id: `loc-${nextId++}`,
      name: folder.path.split("/").pop()!,
      path: folder.path,
      scan: null,
    };
    locations = [...locations, location];
    void simulateScan(location);
    return { location, replaced: [] };
  },

  async listLocations(): Promise<Location[]> {
    await delay(100);
    return locations;
  },

  async removeLocation(id: string): Promise<void> {
    await delay(100);
    locations = locations.filter((l) => l.id !== id);
    files = files.filter((f) => f.locationId !== id);
  },

  async rescanLocation(id: string): Promise<void> {
    const location = locations.find((l) => l.id === id);
    if (location) void simulateScan(location);
  },

  async listFiles(query: FileListQuery = {}): Promise<FileListPage> {
    await delay(100);
    const needle = query.nameContains?.trim().toLowerCase();
    const matches = files
      .filter((f) => !query.locationId || f.locationId === query.locationId)
      .filter((f) => !query.kind || f.kind === query.kind)
      .filter((f) => !needle || f.name.toLowerCase().includes(needle))
      .sort((a, b) => (b.modifiedAt ?? 0) - (a.modifiedAt ?? 0));
    const offset = query.offset ?? 0;
    return { files: matches.slice(offset, offset + (query.limit ?? 100)), total: matches.length };
  },

  async onScanProgress(handler) {
    listeners.add(handler);
    return () => listeners.delete(handler);
  },
};
