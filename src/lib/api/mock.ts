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
  DocumentText,
  Evidence,
  FileKind,
  FileListPage,
  FileListQuery,
  IndexedFile,
  Location,
  RecallApi,
  ScanStatus,
  SearchCapabilities,
  SearchResult,
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
    filesToRead: 0,
    filesRead: 0,
    readFailed: 0,
    passagesToEmbed: 0,
    passagesEmbedded: 0,
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
        status: kind === "image" ? ("pending" as const) : ("indexed" as const),
        error: null,
      })),
    );
  locations = locations.map((l) => (l.id === location.id ? { ...l, fileCount: total } : l));
  const readable = folder.files.filter(([, kind]) => kind !== "image").length;
  setScan({ ...base, state: "reading", filesFound: total, filesProcessed: total, filesToRead: readable });
  await delay(400);
  setScan({
    ...base,
    state: "done",
    filesFound: total,
    filesProcessed: total,
    filesToRead: readable,
    filesRead: readable,
  });
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
      fileCount: 0,
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

  async getDocument(fileId: string): Promise<DocumentText | null> {
    await delay(150);
    const file = files.find((f) => f.id === fileId);
    if (!file || file.status !== "indexed") return null;
    return {
      fileId,
      title: null,
      author: null,
      pageCount: file.kind === "pdf" ? 2 : null,
      wordCount: 42,
      chunkCount: 2,
      pages: (file.kind === "pdf" ? [1, 2] : [null]).map((number) => ({
        number,
        text: `[Mock text] This is placeholder text for ${file.name}${number ? `, page ${number}` : ""}. Real text comes from the backend.`,
      })),
    };
  },

  async search({ query, limit = 20 }): Promise<SearchResult[]> {
    await delay(200);
    // Mock matching: file names containing any word of the query.
    const words = query.toLowerCase().split(/\W+/).filter((w) => w.length > 2);
    return files
      .filter((f) => f.status === "indexed" && words.some((w) => f.name.toLowerCase().includes(w)))
      .slice(0, limit)
      .map((f, i) => {
        const word = words.find((w) => f.name.toLowerCase().includes(w))!;
        return {
          id: `chunk-${f.id}`,
          passageId: f.id,
          fileId: f.id,
          fileName: f.name,
          filePath: f.path,
          fileKind: f.kind,
          page: f.kind === "pdf" ? 2 : null,
          snippet: [
            { text: "[Mock snippet] …text that mentions ", highlight: false },
            { text: word, highlight: true },
            { text: " in a sentence…", highlight: false },
          ],
          relevance: 1 / (i + 1),
          matchReasons: [`Mentions “${word}”`],
        };
      });
  },

  async getSearchCapabilities(): Promise<SearchCapabilities> {
    return {
      semanticSearch: false,
      embeddingModel: null,
      semanticUnavailableReason: "Mock mode: meaning-based search is not simulated.",
    };
  },

  async getEvidence(passageId: string): Promise<Evidence | null> {
    await delay(150);
    const file = files.find((f) => f.id === passageId);
    if (!file) return null;
    return {
      fileId: file.id,
      fileName: file.name,
      filePath: file.path,
      fileKind: file.kind,
      page: file.kind === "pdf" ? 2 : null,
      before: "[Mock text before the passage.] ",
      passage: `[Mock passage] This sentence stands in for the part of ${file.name} that matched.`,
      after: " [Mock text after the passage.]",
    };
  },

  async openFile(fileId: string): Promise<void> {
    await delay(100);
    console.info(`[mock] would open file ${fileId}`);
  },

  async onScanProgress(handler) {
    listeners.add(handler);
    return () => listeners.delete(handler);
  },
};
