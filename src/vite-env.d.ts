/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "true" enables the development-only mock backend. See src/lib/api/client.ts. */
  readonly VITE_RECALL_MOCK?: string;
}
