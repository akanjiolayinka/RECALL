import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api, type FileListQuery } from "@/lib/api/client";

export const FILES_KEY = ["files"];

/** Files found in the library, filtered and paged by the backend. */
export function useFiles(query: FileListQuery) {
  return useQuery({
    queryKey: [...FILES_KEY, query],
    queryFn: () => api.listFiles(query),
    // Keep showing the previous results while new filters load (no flicker).
    placeholderData: keepPreviousData,
  });
}
